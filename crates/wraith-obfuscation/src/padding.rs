//! Packet padding strategies.

use crate::PaddingMode;

/// Padding size classes (bytes)
pub const PADDING_CLASSES: &[usize] = &[64, 256, 512, 1024, 1472, 8960];

/// Select padding class for a payload
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
