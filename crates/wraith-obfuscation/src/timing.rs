//! Timing obfuscation.

use crate::TimingMode;
use std::time::Duration;

/// Calculate inter-packet delay based on timing mode
///
/// Returns a `Duration` representing the delay to insert between packets:
/// - `LowLatency`: Zero delay for maximum throughput
/// - `Moderate`: 100μs fixed delay (future: exponential distribution)
/// - `HighPrivacy`: 5ms fixed delay (future: `HTTPS` timing distribution)
///
/// Future implementations will use randomized delays sampled from realistic
/// traffic distributions to resist timing analysis attacks.
#[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_delay_low_latency() {
        let delay = calculate_delay(TimingMode::LowLatency);
        assert_eq!(delay, Duration::ZERO);
        assert_eq!(delay.as_micros(), 0);
    }

    #[test]
    fn test_calculate_delay_moderate() {
        let delay = calculate_delay(TimingMode::Moderate);
        assert_eq!(delay, Duration::from_micros(100));
        assert_eq!(delay.as_micros(), 100);
    }

    #[test]
    fn test_calculate_delay_high_privacy() {
        let delay = calculate_delay(TimingMode::HighPrivacy);
        assert_eq!(delay, Duration::from_millis(5));
        assert_eq!(delay.as_millis(), 5);
        assert_eq!(delay.as_micros(), 5000);
    }

    #[test]
    fn test_calculate_delay_ordering() {
        // Verify delays are ordered: LowLatency < Moderate < HighPrivacy
        let low = calculate_delay(TimingMode::LowLatency);
        let moderate = calculate_delay(TimingMode::Moderate);
        let high = calculate_delay(TimingMode::HighPrivacy);

        assert!(low < moderate, "LowLatency should be less than Moderate");
        assert!(moderate < high, "Moderate should be less than HighPrivacy");
    }

    #[test]
    fn test_calculate_delay_reasonable_ranges() {
        // LowLatency should be exactly zero
        let low = calculate_delay(TimingMode::LowLatency);
        assert_eq!(low.as_micros(), 0);

        // Moderate should be in sub-millisecond range (< 1ms)
        let moderate = calculate_delay(TimingMode::Moderate);
        assert!(
            moderate.as_micros() < 1000,
            "Moderate delay should be < 1ms"
        );

        // HighPrivacy should be in low millisecond range (< 100ms)
        let high = calculate_delay(TimingMode::HighPrivacy);
        assert!(
            high.as_millis() < 100,
            "HighPrivacy delay should be < 100ms"
        );
    }

    #[test]
    fn test_calculate_delay_deterministic() {
        // All modes should return deterministic values (for now, until randomization is implemented)
        for _ in 0..10 {
            assert_eq!(calculate_delay(TimingMode::LowLatency), Duration::ZERO);
            assert_eq!(
                calculate_delay(TimingMode::Moderate),
                Duration::from_micros(100)
            );
            assert_eq!(
                calculate_delay(TimingMode::HighPrivacy),
                Duration::from_millis(5)
            );
        }
    }

    #[test]
    fn test_calculate_delay_all_modes() {
        // Ensure all modes are covered and don't panic
        let modes = [
            TimingMode::LowLatency,
            TimingMode::Moderate,
            TimingMode::HighPrivacy,
        ];

        for mode in modes {
            let _delay = calculate_delay(mode);
            // Just ensure all modes can be called without panic
        }
    }

    #[test]
    fn test_calculate_delay_moderate_precise() {
        let delay = calculate_delay(TimingMode::Moderate);
        assert_eq!(delay.as_nanos(), 100_000); // 100 microseconds
    }

    #[test]
    fn test_calculate_delay_high_privacy_precise() {
        let delay = calculate_delay(TimingMode::HighPrivacy);
        assert_eq!(delay.as_nanos(), 5_000_000); // 5 milliseconds
    }
}
