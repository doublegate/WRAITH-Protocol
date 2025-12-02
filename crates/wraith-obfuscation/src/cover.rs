//! Cover traffic generation for traffic analysis resistance.

use std::time::{Duration, Instant};

/// Traffic generation distribution
#[derive(Clone, Copy, Debug)]
pub enum TrafficDistribution {
    /// Constant rate
    Constant,
    /// Poisson distribution (models real traffic)
    Poisson {
        /// Rate parameter (events per second)
        lambda: f64,
    },
    /// Uniform random intervals
    Uniform {
        /// Minimum delay in milliseconds
        min_ms: u64,
        /// Maximum delay in milliseconds
        max_ms: u64,
    },
}

/// Cover traffic generator
pub struct CoverTrafficGenerator {
    /// Target packets per second
    rate: f64,
    /// Distribution for timing
    distribution: TrafficDistribution,
    /// Next scheduled send time
    next_send: Instant,
    /// Is generator active
    active: bool,
}

impl CoverTrafficGenerator {
    /// Create a new cover traffic generator
    ///
    /// # Arguments
    ///
    /// * `rate` - Target packets per second
    /// * `distribution` - Timing distribution to use
    #[must_use]
    pub fn new(rate: f64, distribution: TrafficDistribution) -> Self {
        let next_send = Self::calculate_next_send(Instant::now(), rate, distribution);
        Self {
            rate,
            distribution,
            next_send,
            active: true,
        }
    }

    /// Check if it's time to send cover traffic
    #[must_use]
    pub fn should_send(&self) -> bool {
        self.active && Instant::now() >= self.next_send
    }

    /// Get time until next send
    #[must_use]
    pub fn time_until_next(&self) -> Duration {
        let now = Instant::now();
        if now >= self.next_send {
            Duration::from_secs(0)
        } else {
            self.next_send.duration_since(now)
        }
    }

    /// Mark that cover traffic was sent, schedule next
    pub fn mark_sent(&mut self) {
        let now = Instant::now();
        self.next_send = Self::calculate_next_send(now, self.rate, self.distribution);
    }

    /// Generate random padding size
    ///
    /// Returns a padding size between 0 and 256 bytes
    #[must_use]
    pub fn random_pad_size(&self) -> usize {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range(0..=256)
    }

    /// Enable/disable generator
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        if active {
            // Reset next send time when re-enabling
            self.next_send =
                Self::calculate_next_send(Instant::now(), self.rate, self.distribution);
        }
    }

    /// Calculate next send time based on distribution
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    fn calculate_next_send(now: Instant, rate: f64, distribution: TrafficDistribution) -> Instant {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let delay_ms = match distribution {
            TrafficDistribution::Constant => {
                // Fixed interval based on rate
                if rate > 0.0 {
                    // Note: precision loss acceptable for timing
                    (1000.0 / rate) as u64
                } else {
                    1000
                }
            }
            TrafficDistribution::Poisson { lambda } => {
                // Generate exponential inter-arrival time
                let u: f64 = rng.r#gen();
                // Note: precision loss acceptable for timing
                ((-u.ln() / lambda) * 1000.0) as u64
            }
            TrafficDistribution::Uniform { min_ms, max_ms } => rng.gen_range(min_ms..=max_ms),
        };

        now + Duration::from_millis(delay_ms)
    }
}

impl Default for CoverTrafficGenerator {
    fn default() -> Self {
        Self::new(10.0, TrafficDistribution::Constant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cover_traffic_new() {
        let generator = CoverTrafficGenerator::new(10.0, TrafficDistribution::Constant);

        assert!(generator.active);
        assert_eq!(generator.rate, 10.0);
    }

    #[test]
    fn test_cover_traffic_should_send() {
        let mut generator = CoverTrafficGenerator::new(100.0, TrafficDistribution::Constant);

        // Should send initially (next_send is in the past)
        thread::sleep(Duration::from_millis(20));
        assert!(generator.should_send());

        // Mark sent and check immediately (should not send)
        generator.mark_sent();
        assert!(!generator.should_send());
    }

    #[test]
    fn test_cover_traffic_time_until_next() {
        let mut generator = CoverTrafficGenerator::new(10.0, TrafficDistribution::Constant);

        generator.mark_sent();

        let time_until = generator.time_until_next();

        // Should be roughly 100ms (1000ms / 10 pps)
        assert!(time_until.as_millis() > 0);
        assert!(time_until.as_millis() <= 150); // Some slack for timing
    }

    #[test]
    fn test_cover_traffic_mark_sent() {
        let mut generator = CoverTrafficGenerator::new(10.0, TrafficDistribution::Constant);

        let initial_next = generator.next_send;

        thread::sleep(Duration::from_millis(10));

        generator.mark_sent();

        // Next send should be updated
        assert!(generator.next_send > initial_next);
    }

    #[test]
    fn test_cover_traffic_random_pad_size() {
        let generator = CoverTrafficGenerator::new(10.0, TrafficDistribution::Constant);

        for _ in 0..10 {
            let pad_size = generator.random_pad_size();
            assert!(pad_size <= 256);
        }
    }

    #[test]
    fn test_cover_traffic_set_active() {
        let mut generator = CoverTrafficGenerator::new(10.0, TrafficDistribution::Constant);

        assert!(generator.active);

        generator.set_active(false);
        assert!(!generator.active);
        assert!(!generator.should_send());

        generator.set_active(true);
        assert!(generator.active);
    }

    #[test]
    fn test_cover_traffic_constant_distribution() {
        let generator = CoverTrafficGenerator::new(10.0, TrafficDistribution::Constant);

        // With constant rate of 10 pps, interval should be ~100ms
        let time_until = generator.time_until_next();
        assert!(time_until.as_millis() >= 90);
        assert!(time_until.as_millis() <= 110);
    }

    #[test]
    fn test_cover_traffic_uniform_distribution() {
        let generator = CoverTrafficGenerator::new(
            10.0,
            TrafficDistribution::Uniform {
                min_ms: 50,
                max_ms: 150,
            },
        );

        // Time should be within uniform range (with small tolerance for floating point)
        let time_until = generator.time_until_next();
        assert!(time_until.as_millis() >= 45); // Allow slight underflow from floating point
        assert!(time_until.as_millis() <= 160); // Allow slight overflow
    }

    #[test]
    fn test_cover_traffic_poisson_distribution() {
        let generator =
            CoverTrafficGenerator::new(10.0, TrafficDistribution::Poisson { lambda: 0.1 });

        // Poisson should generate some delay
        let time_until = generator.time_until_next();
        assert!(time_until.as_millis() > 0);
    }

    #[test]
    fn test_cover_traffic_default() {
        let generator = CoverTrafficGenerator::default();

        assert_eq!(generator.rate, 10.0);
        assert!(generator.active);
        assert!(matches!(
            generator.distribution,
            TrafficDistribution::Constant
        ));
    }

    #[test]
    fn test_cover_traffic_inactive_should_not_send() {
        let mut generator = CoverTrafficGenerator::new(10.0, TrafficDistribution::Constant);

        generator.set_active(false);

        thread::sleep(Duration::from_millis(200));

        assert!(!generator.should_send());
    }

    #[test]
    fn test_cover_traffic_reactivation_resets_timer() {
        let mut generator = CoverTrafficGenerator::new(10.0, TrafficDistribution::Constant);

        let initial_next = generator.next_send;

        thread::sleep(Duration::from_millis(10));

        generator.set_active(false);
        generator.set_active(true);

        // Next send should be updated to future time
        assert!(generator.next_send > initial_next);
    }
}
