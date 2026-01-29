//! Packet padding strategies for traffic analysis resistance.
//!
//! This module provides a comprehensive padding engine with multiple modes
//! to obscure message sizes and defeat traffic analysis attacks.

use rand::Rng;
use rand_distr::{Distribution, Geometric};

/// Packet padding modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddingMode {
    /// No padding
    None,
    /// Round to power of 2 (minimum 128 bytes)
    PowerOfTwo,
    /// Fixed size classes (128, 256, 512, 1024, 2048, 4096, 8192, 16384)
    SizeClasses,
    /// Constant rate padding (always max size)
    ConstantRate,
    /// Statistical padding (random from geometric distribution)
    Statistical,
}

/// Padding size classes (bytes).
/// Adding 256 and 2048 reduces maximum overhead from 297% to 99%
/// for messages in the 129-256 byte range (common for chat messages).
const SIZE_CLASSES: &[usize] = &[128, 256, 512, 1024, 2048, 4096, 8192, 16384];

/// Packet padding engine
///
/// Provides various padding strategies to obscure message sizes and
/// defeat traffic analysis. Supports deterministic and statistical modes.
///
/// # Examples
///
/// ```
/// use wraith_obfuscation::padding::{PaddingEngine, PaddingMode};
///
/// let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
/// let padded_size = engine.padded_size(100);
/// assert_eq!(padded_size, 128); // Rounds up to smallest class
/// ```
pub struct PaddingEngine {
    mode: PaddingMode,
    rng: rand::rngs::ThreadRng,
}

impl PaddingEngine {
    /// Create a new padding engine with the specified mode
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::padding::{PaddingEngine, PaddingMode};
    ///
    /// let engine = PaddingEngine::new(PaddingMode::SizeClasses);
    /// ```
    #[must_use]
    pub fn new(mode: PaddingMode) -> Self {
        Self {
            mode,
            rng: rand::thread_rng(),
        }
    }

    /// Calculate padded size for given plaintext length
    ///
    /// Returns the target size after padding based on the engine's mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::padding::{PaddingEngine, PaddingMode};
    ///
    /// let mut engine = PaddingEngine::new(PaddingMode::PowerOfTwo);
    /// assert_eq!(engine.padded_size(100), 128);
    /// assert_eq!(engine.padded_size(129), 256);
    /// ```
    pub fn padded_size(&mut self, plaintext_len: usize) -> usize {
        match self.mode {
            PaddingMode::None => plaintext_len,

            PaddingMode::PowerOfTwo => {
                // Round up to next power of 2
                let next_pow2 = plaintext_len.next_power_of_two();
                next_pow2.max(128) // Minimum 128 bytes
            }

            PaddingMode::SizeClasses => {
                // Binary search for smallest size class that fits
                let idx = SIZE_CLASSES.partition_point(|&s| s < plaintext_len);
                if idx < SIZE_CLASSES.len() {
                    SIZE_CLASSES[idx]
                } else {
                    *SIZE_CLASSES.last().unwrap()
                }
            }

            PaddingMode::ConstantRate => {
                // Always pad to maximum size
                *SIZE_CLASSES.last().unwrap()
            }

            PaddingMode::Statistical => {
                // Use geometric distribution for realistic padding
                let mean = 1.5; // Average padding multiplier
                let p = 1.0 / mean;
                let geo = Geometric::new(p).unwrap();

                let extra_padding = (geo.sample(&mut self.rng) as usize) * 128;
                let padded = plaintext_len + extra_padding;

                // Clamp to reasonable bounds
                padded.clamp(128, 16384)
            }
        }
    }

    /// Add padding to buffer
    ///
    /// Extends the buffer with random padding bytes to reach the target size.
    /// If the buffer is already at or exceeds the target size, no padding is added.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::padding::{PaddingEngine, PaddingMode};
    ///
    /// let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
    /// let mut buffer = b"hello".to_vec();
    /// engine.pad(&mut buffer, 128);
    /// assert_eq!(buffer.len(), 128);
    /// ```
    pub fn pad(&mut self, buffer: &mut Vec<u8>, target_size: usize) {
        if buffer.len() >= target_size {
            return;
        }

        // Add padding bytes (random data)
        let padding_start = buffer.len();
        buffer.resize(target_size, 0);
        self.rng.fill(&mut buffer[padding_start..]);
    }

    /// Remove padding from buffer (returns original length)
    ///
    /// Returns a slice containing only the original data, excluding padding.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::padding::{PaddingEngine, PaddingMode};
    ///
    /// let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
    /// let original = b"hello";
    /// let mut buffer = original.to_vec();
    /// engine.pad(&mut buffer, 128);
    ///
    /// let unpadded = engine.unpad(&buffer, original.len());
    /// assert_eq!(unpadded, original);
    /// ```
    #[must_use]
    pub fn unpad<'a>(&self, buffer: &'a [u8], original_len: usize) -> &'a [u8] {
        &buffer[..original_len.min(buffer.len())]
    }

    /// Calculate overhead percentage
    ///
    /// Returns the percentage overhead introduced by padding.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::padding::{PaddingEngine, PaddingMode};
    ///
    /// let engine = PaddingEngine::new(PaddingMode::SizeClasses);
    /// let overhead = engine.overhead(100);
    /// assert!((overhead - 28.0).abs() < 1.0); // 100 -> 128 = 28% overhead
    /// ```
    #[must_use]
    pub fn overhead(&self, plaintext_len: usize) -> f64 {
        let padded = self.padded_size_const(plaintext_len);
        if plaintext_len == 0 {
            return 0.0;
        }
        ((padded - plaintext_len) as f64 / plaintext_len as f64) * 100.0
    }

    /// Get the current padding mode
    #[must_use]
    pub const fn mode(&self) -> PaddingMode {
        self.mode
    }

    /// Set a new padding mode
    pub fn set_mode(&mut self, mode: PaddingMode) {
        self.mode = mode;
    }

    // Calculate padded size (const version for overhead calculation)
    fn padded_size_const(&self, plaintext_len: usize) -> usize {
        match self.mode {
            PaddingMode::None => plaintext_len,
            PaddingMode::PowerOfTwo => plaintext_len.next_power_of_two().max(128),
            PaddingMode::SizeClasses => {
                let idx = SIZE_CLASSES.partition_point(|&s| s < plaintext_len);
                if idx < SIZE_CLASSES.len() {
                    SIZE_CLASSES[idx]
                } else {
                    *SIZE_CLASSES.last().unwrap()
                }
            }
            PaddingMode::ConstantRate => *SIZE_CLASSES.last().unwrap(),
            PaddingMode::Statistical => {
                // Average case for statistical mode
                ((plaintext_len as f64) * 1.5) as usize
            }
        }
    }
}

impl Default for PaddingEngine {
    fn default() -> Self {
        Self::new(PaddingMode::SizeClasses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_padding() {
        let mut engine = PaddingEngine::new(PaddingMode::None);
        assert_eq!(engine.padded_size(100), 100);
        assert_eq!(engine.padded_size(1000), 1000);
        assert_eq!(engine.padded_size(0), 0);
    }

    #[test]
    fn test_power_of_two() {
        let mut engine = PaddingEngine::new(PaddingMode::PowerOfTwo);
        assert_eq!(engine.padded_size(100), 128);
        assert_eq!(engine.padded_size(128), 128);
        assert_eq!(engine.padded_size(129), 256);
        assert_eq!(engine.padded_size(1000), 1024);
        assert_eq!(engine.padded_size(1), 128); // Minimum 128
    }

    #[test]
    fn test_size_classes() {
        let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
        assert_eq!(engine.padded_size(100), 128);
        assert_eq!(engine.padded_size(128), 128);
        assert_eq!(engine.padded_size(129), 256);
        assert_eq!(engine.padded_size(500), 512);
        assert_eq!(engine.padded_size(513), 1024);
        assert_eq!(engine.padded_size(1000), 1024);
        assert_eq!(engine.padded_size(1025), 2048);
        assert_eq!(engine.padded_size(5000), 8192);
        assert_eq!(engine.padded_size(20000), 16384);
    }

    #[test]
    fn test_constant_rate() {
        let mut engine = PaddingEngine::new(PaddingMode::ConstantRate);
        assert_eq!(engine.padded_size(100), 16384);
        assert_eq!(engine.padded_size(1000), 16384);
        assert_eq!(engine.padded_size(8000), 16384);
        assert_eq!(engine.padded_size(16384), 16384);
    }

    #[test]
    fn test_statistical_padding() {
        let mut engine = PaddingEngine::new(PaddingMode::Statistical);

        // Statistical mode should add variable padding
        let mut sizes = Vec::new();
        for _ in 0..100 {
            sizes.push(engine.padded_size(100));
        }

        // Should have some variation
        let min_size = *sizes.iter().min().unwrap();
        let max_size = *sizes.iter().max().unwrap();
        assert!(max_size > min_size, "Statistical mode should vary");

        // All sizes should be >= original
        for size in sizes {
            assert!(size >= 100);
            assert!(size <= 16384); // Clamped to max
        }
    }

    #[test]
    fn test_padding_overhead() {
        let engine = PaddingEngine::new(PaddingMode::SizeClasses);

        // 100 bytes -> 128 bytes = 28% overhead
        let overhead = engine.overhead(100);
        assert!((overhead - 28.0).abs() < 1.0);

        // 500 bytes -> 512 bytes = 2.4% overhead
        let overhead = engine.overhead(500);
        assert!((overhead - 2.4).abs() < 1.0);

        // 1000 bytes -> 1024 bytes = 2.4% overhead
        let overhead = engine.overhead(1000);
        assert!((overhead - 2.4).abs() < 1.0);
    }

    #[test]
    fn test_pad_unpad_roundtrip() {
        let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let original = b"hello world";
        let original_len = original.len();

        let mut buffer = original.to_vec();
        let target_size = engine.padded_size(original_len);

        engine.pad(&mut buffer, target_size);
        assert_eq!(buffer.len(), target_size);

        let unpadded = engine.unpad(&buffer, original_len);
        assert_eq!(unpadded, original);
    }

    #[test]
    fn test_pad_larger_buffer() {
        let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let mut buffer = vec![0u8; 200];

        // Padding to smaller size should not modify buffer
        let original_len = buffer.len();
        engine.pad(&mut buffer, 128);
        assert_eq!(buffer.len(), original_len);
    }

    #[test]
    fn test_random_padding_data() {
        let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let original = b"test";

        let mut buffer1 = original.to_vec();
        let mut buffer2 = original.to_vec();

        let target_size = engine.padded_size(original.len());

        engine.pad(&mut buffer1, target_size);
        engine.pad(&mut buffer2, target_size);

        // Original data should be same
        assert_eq!(&buffer1[..original.len()], original);
        assert_eq!(&buffer2[..original.len()], original);

        // Padding should be different (random)
        assert_ne!(buffer1, buffer2);
    }

    #[test]
    fn test_mode_getter_setter() {
        let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
        assert_eq!(engine.mode(), PaddingMode::SizeClasses);

        engine.set_mode(PaddingMode::PowerOfTwo);
        assert_eq!(engine.mode(), PaddingMode::PowerOfTwo);
    }

    #[test]
    fn test_default_engine() {
        let engine = PaddingEngine::default();
        assert_eq!(engine.mode(), PaddingMode::SizeClasses);
    }

    #[test]
    fn test_size_classes_constant() {
        assert_eq!(SIZE_CLASSES.len(), 8);
        assert_eq!(
            SIZE_CLASSES,
            &[128, 256, 512, 1024, 2048, 4096, 8192, 16384]
        );
    }

    #[test]
    fn test_overhead_zero_length() {
        let engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let overhead = engine.overhead(0);
        assert_eq!(overhead, 0.0);
    }

    #[test]
    fn test_unpad_empty_buffer() {
        let engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let buffer = b"";
        let unpadded = engine.unpad(buffer, 0);
        assert_eq!(unpadded, b"");
    }

    #[test]
    fn test_unpad_exact_length() {
        let engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let buffer = b"hello";
        let unpadded = engine.unpad(buffer, 5);
        assert_eq!(unpadded, b"hello");
    }

    #[test]
    fn test_unpad_longer_than_buffer() {
        let engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let buffer = b"hello";
        // Request more than buffer has - should clamp
        let unpadded = engine.unpad(buffer, 100);
        assert_eq!(unpadded, b"hello");
    }

    #[test]
    fn test_all_modes_enumeration() {
        // Ensure all modes can be constructed and used
        let modes = [
            PaddingMode::None,
            PaddingMode::PowerOfTwo,
            PaddingMode::SizeClasses,
            PaddingMode::ConstantRate,
            PaddingMode::Statistical,
        ];

        for mode in modes {
            let mut engine = PaddingEngine::new(mode);
            let _size = engine.padded_size(100);
            // Just ensure all modes work without panic
        }
    }

    #[test]
    fn test_overhead_constant_rate() {
        let engine = PaddingEngine::new(PaddingMode::ConstantRate);

        // 100 bytes -> 16384 bytes
        let overhead = engine.overhead(100);
        assert!((overhead - 16284.0).abs() < 1.0);
    }

    #[test]
    fn test_power_of_two_edge_cases() {
        let mut engine = PaddingEngine::new(PaddingMode::PowerOfTwo);

        // Edge cases around power of 2 boundaries
        assert_eq!(engine.padded_size(64), 128); // Below minimum
        assert_eq!(engine.padded_size(127), 128); // Just below 128
        assert_eq!(engine.padded_size(255), 256); // Just below 256
        assert_eq!(engine.padded_size(256), 256); // Exact power of 2
        assert_eq!(engine.padded_size(257), 512); // Just over 256
    }

    #[test]
    fn test_size_classes_boundaries() {
        let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);

        // Test exact boundaries
        assert_eq!(engine.padded_size(128), 128);
        assert_eq!(engine.padded_size(256), 256);
        assert_eq!(engine.padded_size(512), 512);
        assert_eq!(engine.padded_size(1024), 1024);
        assert_eq!(engine.padded_size(2048), 2048);
        assert_eq!(engine.padded_size(4096), 4096);
        assert_eq!(engine.padded_size(8192), 8192);
        assert_eq!(engine.padded_size(16384), 16384);

        // Test just over boundaries
        assert_eq!(engine.padded_size(129), 256);
        assert_eq!(engine.padded_size(257), 512);
        assert_eq!(engine.padded_size(513), 1024);
        assert_eq!(engine.padded_size(1025), 2048);
        assert_eq!(engine.padded_size(2049), 4096);
        assert_eq!(engine.padded_size(4097), 8192);
        assert_eq!(engine.padded_size(8193), 16384);
    }

    #[test]
    fn test_statistical_bounds() {
        let mut engine = PaddingEngine::new(PaddingMode::Statistical);

        // Statistical mode should always return >= 128
        for _ in 0..50 {
            let size = engine.padded_size(50);
            assert!(size >= 128);
        }

        // Should clamp to max
        for _ in 0..50 {
            let size = engine.padded_size(20000);
            assert_eq!(size, 16384);
        }
    }

    #[test]
    fn test_pad_preserves_original_data() {
        let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let original = b"The quick brown fox jumps over the lazy dog";
        let mut buffer = original.to_vec();

        let target_size = engine.padded_size(original.len());
        engine.pad(&mut buffer, target_size);

        // Original data should be unchanged at the beginning
        assert_eq!(&buffer[..original.len()], original);
    }

    #[test]
    fn test_mode_comparison() {
        // Test that different modes produce different sizes
        let mut none_engine = PaddingEngine::new(PaddingMode::None);
        let mut pow2_engine = PaddingEngine::new(PaddingMode::PowerOfTwo);
        let mut class_engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let mut const_engine = PaddingEngine::new(PaddingMode::ConstantRate);

        let input = 100;

        assert_eq!(none_engine.padded_size(input), 100);
        assert_eq!(pow2_engine.padded_size(input), 128);
        assert_eq!(class_engine.padded_size(input), 128);
        assert_eq!(const_engine.padded_size(input), 16384);
    }
}
