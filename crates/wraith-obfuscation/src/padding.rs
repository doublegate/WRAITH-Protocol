//! Packet padding strategies.

use crate::PaddingMode;

/// Padding size classes (bytes)
pub const PADDING_CLASSES: &[usize] = &[64, 256, 512, 1024, 1472, 8960];

/// Select padding class for a payload
///
/// Selects the smallest padding class that can accommodate the payload plus
/// frame overhead (28-byte header + 16-byte auth tag). In `Performance` mode,
/// selection is deterministic. In `Privacy` and `Stealth` modes, future
/// implementations will use randomized selection for traffic analysis resistance.
#[must_use]
pub fn select_padding_class(payload_len: usize, mode: PaddingMode) -> usize {
    let header_overhead = 28 + 16; // Frame header + auth tag

    match mode {
        PaddingMode::Performance => PADDING_CLASSES
            .iter()
            .find(|&&size| size >= payload_len + header_overhead)
            .copied()
            .unwrap_or(8960),
        PaddingMode::Privacy | PaddingMode::Stealth => {
            // TODO: Implement random/statistical selection
            PADDING_CLASSES
                .iter()
                .find(|&&size| size >= payload_len + header_overhead)
                .copied()
                .unwrap_or(8960)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_padding_classes_constant() {
        // Verify PADDING_CLASSES has expected values
        assert_eq!(PADDING_CLASSES.len(), 6);
        assert_eq!(PADDING_CLASSES[0], 64);
        assert_eq!(PADDING_CLASSES[1], 256);
        assert_eq!(PADDING_CLASSES[2], 512);
        assert_eq!(PADDING_CLASSES[3], 1024);
        assert_eq!(PADDING_CLASSES[4], 1472);
        assert_eq!(PADDING_CLASSES[5], 8960);
    }

    #[test]
    fn test_select_padding_class_performance_zero() {
        // Zero-length payload should fit in smallest class (64 bytes)
        let size = select_padding_class(0, PaddingMode::Performance);
        assert_eq!(size, 64);
    }

    #[test]
    fn test_select_padding_class_performance_small() {
        // Small payload (10 bytes) + overhead (44 bytes) = 54 bytes -> fits in 64
        let size = select_padding_class(10, PaddingMode::Performance);
        assert_eq!(size, 64);
    }

    #[test]
    fn test_select_padding_class_performance_boundary() {
        // Payload exactly at boundary (20 bytes) + overhead (44 bytes) = 64 bytes
        let size = select_padding_class(20, PaddingMode::Performance);
        assert_eq!(size, 64);
    }

    #[test]
    fn test_select_padding_class_performance_next_class() {
        // Payload slightly over boundary (21 bytes) + overhead (44 bytes) = 65 bytes -> 256
        let size = select_padding_class(21, PaddingMode::Performance);
        assert_eq!(size, 256);
    }

    #[test]
    fn test_select_padding_class_performance_medium() {
        // Medium payload (200 bytes) + overhead (44 bytes) = 244 bytes -> fits in 256
        let size = select_padding_class(200, PaddingMode::Performance);
        assert_eq!(size, 256);
    }

    #[test]
    fn test_select_padding_class_performance_large() {
        // Large payload (1400 bytes) + overhead (44 bytes) = 1444 bytes -> fits in 1472
        let size = select_padding_class(1400, PaddingMode::Performance);
        assert_eq!(size, 1472);
    }

    #[test]
    fn test_select_padding_class_performance_max() {
        // Maximum payload (8000 bytes) + overhead (44 bytes) = 8044 bytes -> fits in 8960
        let size = select_padding_class(8000, PaddingMode::Performance);
        assert_eq!(size, 8960);
    }

    #[test]
    fn test_select_padding_class_performance_overflow() {
        // Payload larger than max class -> returns 8960
        let size = select_padding_class(10000, PaddingMode::Performance);
        assert_eq!(size, 8960);
    }

    #[test]
    fn test_select_padding_class_privacy_small() {
        // Privacy mode should select same as performance (for now, until randomization is implemented)
        let size = select_padding_class(10, PaddingMode::Privacy);
        assert_eq!(size, 64);
    }

    #[test]
    fn test_select_padding_class_privacy_medium() {
        let size = select_padding_class(200, PaddingMode::Privacy);
        assert_eq!(size, 256);
    }

    #[test]
    fn test_select_padding_class_stealth_small() {
        // Stealth mode should select same as performance (for now, until randomization is implemented)
        let size = select_padding_class(10, PaddingMode::Stealth);
        assert_eq!(size, 64);
    }

    #[test]
    fn test_select_padding_class_stealth_medium() {
        let size = select_padding_class(200, PaddingMode::Stealth);
        assert_eq!(size, 256);
    }

    #[test]
    fn test_select_padding_class_all_modes_consistency() {
        // All modes should return same result for now (until randomization is implemented)
        let payload_sizes = [0, 10, 100, 500, 1000, 5000, 10000];

        for &payload in &payload_sizes {
            let perf = select_padding_class(payload, PaddingMode::Performance);
            let priv_size = select_padding_class(payload, PaddingMode::Privacy);
            let stealth = select_padding_class(payload, PaddingMode::Stealth);

            assert_eq!(
                perf, priv_size,
                "Performance and Privacy should match for payload {}",
                payload
            );
            assert_eq!(
                perf, stealth,
                "Performance and Stealth should match for payload {}",
                payload
            );
        }
    }

    #[test]
    fn test_select_padding_class_header_overhead() {
        // Verify the header overhead calculation (28 + 16 = 44 bytes)
        let header_overhead = 28 + 16;
        assert_eq!(header_overhead, 44);

        // A payload of exactly (64 - 44) = 20 bytes should fit in 64
        let size = select_padding_class(20, PaddingMode::Performance);
        assert_eq!(size, 64);

        // A payload of (64 - 44 + 1) = 21 bytes should require 256
        let size = select_padding_class(21, PaddingMode::Performance);
        assert_eq!(size, 256);
    }

    #[test]
    fn test_select_padding_class_mtu_typical() {
        // Typical MTU payload of 1428 bytes (1500 - 20 IP - 8 UDP - 44 overhead)
        let size = select_padding_class(1428, PaddingMode::Performance);
        assert_eq!(size, 1472);
    }
}
