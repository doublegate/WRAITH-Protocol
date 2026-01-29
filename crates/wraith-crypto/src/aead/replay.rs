//! Replay protection using sliding window.
//!
//! Provides protection against replay attacks by tracking seen packet sequence numbers.
//! Uses a 1024-bit window for efficient out-of-order packet handling.

use subtle::ConstantTimeEq;

/// Replay protection using sliding window.
///
/// Tracks seen packet sequence numbers to prevent replay attacks.
/// Uses a 1024-bit window for efficient out-of-order packet handling.
///
/// # Security
///
/// A 1024-packet window provides tolerance for:
/// - High packet loss scenarios (>10% loss)
/// - Severe packet reordering (network path changes, ECMP)
/// - Bursty traffic patterns (VPN reconnects, mobile handoffs)
/// - High-throughput scenarios (~300 us coverage at 40 Gbps)
#[derive(Clone)]
pub struct ReplayProtection {
    /// Maximum sequence number seen
    max_seq: u64,
    /// Sliding window bitmap (1024 bits = 1024 packets, stored as 16 × 64-bit words)
    window: [u64; 16],
}

impl ReplayProtection {
    /// Size of the replay protection window (1024 packets)
    pub const WINDOW_SIZE: u64 = 1024;

    /// Number of 64-bit words in the window bitmap
    const WINDOW_WORDS: usize = (Self::WINDOW_SIZE / 64) as usize;

    /// Create a new replay protection window
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_seq: 0,
            window: [0; Self::WINDOW_WORDS],
        }
    }

    /// Check if a sequence number is acceptable and update the window.
    ///
    /// Returns `true` if the packet should be accepted (not a replay).
    /// Returns `false` if the packet is a replay or too old.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut rp = ReplayProtection::new();
    ///
    /// assert!(rp.check_and_update(1)); // First packet
    /// assert!(!rp.check_and_update(1)); // Replay - rejected
    /// assert!(rp.check_and_update(2)); // Next packet
    /// assert!(rp.check_and_update(65)); // Jump ahead
    /// assert!(!rp.check_and_update(1)); // Too old - rejected
    /// ```
    pub fn check_and_update(&mut self, seq: u64) -> bool {
        // Packet is too old (beyond window)
        // Use <= to prevent bit_position from being exactly WINDOW_SIZE (64), which would overflow
        if seq + Self::WINDOW_SIZE <= self.max_seq {
            return false;
        }

        // Packet is newer than max_seq (advance window)
        if seq > self.max_seq {
            let shift = seq - self.max_seq;

            if shift >= Self::WINDOW_SIZE {
                // Shift is >= window size, reset window completely
                self.window = [0; Self::WINDOW_WORDS];
                self.window[0] = 1; // Mark bit 0 as seen
            } else {
                // Shift window left by shift bits
                self.shift_window_left(shift);
                // Mark bit 0 as seen (current max_seq position)
                self.window[0] |= 1;
            }

            self.max_seq = seq;
            return true;
        }

        // Packet is within window (seq <= max_seq)
        let bit_position = self.max_seq - seq;

        // Determine which 64-bit word and which bit within it
        let word_index = (bit_position / 64) as usize;
        let bit_index = bit_position % 64;

        // Check if already seen (constant-time comparison for side-channel resistance)
        let bit_mask = 1u64 << bit_index;
        let is_seen = self.window[word_index] & bit_mask;

        // Use constant-time comparison to prevent timing attacks
        // that could leak information about the replay window state
        if is_seen.ct_ne(&0u64).into() {
            return false; // Replay detected
        }

        // Mark as seen
        self.window[word_index] |= bit_mask;
        true
    }

    /// Get the maximum sequence number seen
    #[must_use]
    pub fn max_seq(&self) -> u64 {
        self.max_seq
    }

    /// Reset the replay protection window
    pub fn reset(&mut self) {
        self.max_seq = 0;
        self.window = [0; Self::WINDOW_WORDS];
    }

    /// Shift the window left by `shift` bits (internal helper).
    ///
    /// Implements multi-word left shift for the window bitmap.
    fn shift_window_left(&mut self, shift: u64) {
        if shift == 0 {
            return;
        }

        if shift >= Self::WINDOW_SIZE {
            // Complete shift-out
            self.window = [0; Self::WINDOW_WORDS];
            return;
        }

        let word_shift = (shift / 64) as usize;
        let bit_shift = (shift % 64) as u32;

        if bit_shift == 0 {
            // Word-aligned shift
            for i in (word_shift..Self::WINDOW_WORDS).rev() {
                self.window[i] = self.window[i - word_shift];
            }
            for i in 0..word_shift {
                self.window[i] = 0;
            }
        } else {
            // Bit-level shift across word boundaries
            for i in (word_shift + 1..Self::WINDOW_WORDS).rev() {
                self.window[i] = (self.window[i - word_shift] << bit_shift)
                    | (self.window[i - word_shift - 1] >> (64 - bit_shift));
            }
            self.window[word_shift] = self.window[0] << bit_shift;
            for i in 0..word_shift {
                self.window[i] = 0;
            }
        }
    }
}

impl Default for ReplayProtection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_protection_basic() {
        let mut rp = ReplayProtection::new();

        // First packet should be accepted
        assert!(rp.check_and_update(1));
        assert_eq!(rp.max_seq(), 1);

        // Replay should be rejected
        assert!(!rp.check_and_update(1));

        // Next packet should be accepted
        assert!(rp.check_and_update(2));
        assert_eq!(rp.max_seq(), 2);
    }

    #[test]
    fn test_replay_protection_out_of_order() {
        let mut rp = ReplayProtection::new();

        assert!(rp.check_and_update(10));
        assert!(rp.check_and_update(8)); // Out of order but within window
        assert!(rp.check_and_update(9)); // Fill in the gap
        assert!(!rp.check_and_update(8)); // Replay
    }

    #[test]
    fn test_replay_protection_window_boundary() {
        let mut rp = ReplayProtection::new();

        assert!(rp.check_and_update(1));
        assert!(rp.check_and_update(256)); // Within window

        // Packet 1 is now at the edge of the window
        assert!(!rp.check_and_update(1)); // Should still reject (already seen)

        assert!(rp.check_and_update(257)); // Advance window
        // Now packet 1 is outside the window (too old)
        // This would be rejected due to age, not duplicate detection
    }

    #[test]
    fn test_replay_protection_large_jump() {
        let mut rp = ReplayProtection::new();

        assert!(rp.check_and_update(1));
        assert!(rp.check_and_update(1000)); // Large jump, resets window

        // Old packet should be rejected (too old)
        assert!(!rp.check_and_update(1));
    }

    #[test]
    fn test_replay_protection_reset() {
        let mut rp = ReplayProtection::new();

        assert!(rp.check_and_update(100));
        assert_eq!(rp.max_seq(), 100);

        rp.reset();

        assert_eq!(rp.max_seq(), 0);
        assert!(rp.check_and_update(1)); // Should accept after reset
    }

    #[test]
    fn test_replay_256_bit_window() {
        let mut rp = ReplayProtection::new();

        // Fill the entire 256-bit window
        for i in 1..=256 {
            assert!(rp.check_and_update(i), "Failed to accept seq {}", i);
        }

        // All should be replays now
        for i in 1..=256 {
            assert!(!rp.check_and_update(i), "Should reject replay of seq {}", i);
        }

        // Advance window
        assert!(rp.check_and_update(257));

        // Packet 1 is now out of window (too old)
        assert!(!rp.check_and_update(1));
    }

    #[test]
    fn test_replay_shift_boundary_conditions() {
        let mut rp = ReplayProtection::new();

        // Test word-aligned shift (64 bits)
        assert!(rp.check_and_update(1));
        assert!(rp.check_and_update(65)); // Shift by exactly 64

        // Test non-aligned shift
        let mut rp2 = ReplayProtection::new();
        assert!(rp2.check_and_update(1));
        assert!(rp2.check_and_update(100)); // Shift by 99 (crosses word boundary)
    }
}
