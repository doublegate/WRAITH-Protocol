//! Timing obfuscation.

use crate::TimingMode;
use std::time::Duration;

/// Calculate inter-packet delay based on timing mode
pub fn calculate_delay(mode: TimingMode) -> Duration {
    match mode {
        TimingMode::LowLatency => Duration::ZERO,
        TimingMode::Moderate => {
            // TODO: Implement exponential distribution
            Duration::from_micros(100)
        }
        TimingMode::HighPrivacy => {
            // TODO: Sample from HTTPS timing distribution
            Duration::from_millis(5)
        }
    }
}
