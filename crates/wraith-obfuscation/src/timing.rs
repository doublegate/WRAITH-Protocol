//! Timing obfuscation for traffic analysis resistance.
//!
//! Provides various timing strategies to add jitter and prevent
//! timing correlation attacks.

use rand_distr::{Distribution, Exp, Normal};
use std::time::{Duration, Instant};

/// Timing obfuscation modes
#[derive(Debug, Clone, Copy)]
pub enum TimingMode {
    /// No timing obfuscation
    None,
    /// Fixed delay
    Fixed(Duration),
    /// Uniform random delay
    Uniform {
        /// Minimum delay
        min: Duration,
        /// Maximum delay
        max: Duration,
    },
    /// Normal distribution
    Normal {
        /// Mean delay
        mean: Duration,
        /// Standard deviation
        stddev: Duration,
    },
    /// Exponential distribution
    Exponential {
        /// Mean delay
        mean: Duration,
    },
}

/// Timing obfuscation engine
///
/// Generates delays according to various statistical distributions
/// to resist timing correlation attacks.
///
/// # Examples
///
/// ```
/// use wraith_obfuscation::timing::{TimingObfuscator, TimingMode};
/// use std::time::Duration;
///
/// let mut obfuscator = TimingObfuscator::new(
///     TimingMode::Fixed(Duration::from_millis(10))
/// );
///
/// let delay = obfuscator.next_delay();
/// assert_eq!(delay, Duration::from_millis(10));
/// ```
pub struct TimingObfuscator {
    mode: TimingMode,
    rng: rand::rngs::ThreadRng,
}

impl TimingObfuscator {
    /// Create a new timing obfuscator
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::timing::{TimingObfuscator, TimingMode};
    /// use std::time::Duration;
    ///
    /// let obfuscator = TimingObfuscator::new(TimingMode::None);
    /// ```
    #[must_use]
    pub fn new(mode: TimingMode) -> Self {
        Self {
            mode,
            rng: rand::thread_rng(),
        }
    }

    /// Calculate delay before sending next packet
    ///
    /// Returns a `Duration` sampled from the configured distribution.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::timing::{TimingObfuscator, TimingMode};
    /// use std::time::Duration;
    ///
    /// let mut obfuscator = TimingObfuscator::new(
    ///     TimingMode::Uniform {
    ///         min: Duration::from_millis(5),
    ///         max: Duration::from_millis(15),
    ///     }
    /// );
    ///
    /// let delay = obfuscator.next_delay();
    /// assert!(delay >= Duration::from_millis(5));
    /// assert!(delay <= Duration::from_millis(15));
    /// ```
    pub fn next_delay(&mut self) -> Duration {
        use rand::Rng;
        match self.mode {
            TimingMode::None => Duration::from_micros(0),

            TimingMode::Fixed(delay) => delay,

            TimingMode::Uniform { min, max } => {
                let min_us = min.as_micros() as u64;
                let max_us = max.as_micros() as u64;
                if min_us >= max_us {
                    return min;
                }
                let delay_us = self.rng.gen_range(min_us..=max_us);
                Duration::from_micros(delay_us)
            }

            TimingMode::Normal { mean, stddev } => {
                let mean_us = mean.as_micros() as f64;
                let stddev_us = stddev.as_micros() as f64;

                let normal = Normal::new(mean_us, stddev_us).unwrap();
                let delay_us = normal.sample(&mut self.rng).max(0.0) as u64;

                Duration::from_micros(delay_us)
            }

            TimingMode::Exponential { mean } => {
                let mean_us = mean.as_micros() as f64;
                let lambda = 1.0 / mean_us;

                let exp = Exp::new(lambda).unwrap();
                let delay_us = exp.sample(&mut self.rng) as u64;

                Duration::from_micros(delay_us)
            }
        }
    }

    /// Sleep for obfuscated delay
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::timing::{TimingObfuscator, TimingMode};
    /// use std::time::Duration;
    ///
    /// let mut obfuscator = TimingObfuscator::new(
    ///     TimingMode::Fixed(Duration::from_millis(1))
    /// );
    ///
    /// obfuscator.sleep(); // Sleeps for 1ms
    /// ```
    pub fn sleep(&mut self) {
        let delay = self.next_delay();
        if delay > Duration::from_micros(0) {
            std::thread::sleep(delay);
        }
    }

    /// Get the current timing mode
    #[must_use]
    pub const fn mode(&self) -> &TimingMode {
        &self.mode
    }

    /// Set a new timing mode
    pub fn set_mode(&mut self, mode: TimingMode) {
        self.mode = mode;
    }
}

impl Default for TimingObfuscator {
    fn default() -> Self {
        Self::new(TimingMode::None)
    }
}

/// Traffic shaping to mimic specific patterns
///
/// Enforces a target packet rate by introducing delays between sends.
///
/// # Examples
///
/// ```
/// use wraith_obfuscation::timing::TrafficShaper;
///
/// let mut shaper = TrafficShaper::new(100.0); // 100 packets per second
/// ```
pub struct TrafficShaper {
    target_rate: f64, // packets per second
    last_send: Instant,
}

impl TrafficShaper {
    /// Create a new traffic shaper
    ///
    /// # Arguments
    ///
    /// * `packets_per_second` - Target packet rate
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::timing::TrafficShaper;
    ///
    /// let shaper = TrafficShaper::new(100.0);
    /// ```
    #[must_use]
    pub fn new(packets_per_second: f64) -> Self {
        Self {
            target_rate: packets_per_second,
            last_send: Instant::now(),
        }
    }

    /// Wait until next packet should be sent
    ///
    /// Sleeps until enough time has elapsed to maintain the target rate.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wraith_obfuscation::timing::TrafficShaper;
    ///
    /// let mut shaper = TrafficShaper::new(10.0); // 10 pps
    /// shaper.wait_for_next(); // Waits ~100ms
    /// ```
    pub fn wait_for_next(&mut self) {
        let interval = Duration::from_secs_f64(1.0 / self.target_rate);
        let elapsed = self.last_send.elapsed();

        if elapsed < interval {
            std::thread::sleep(interval - elapsed);
        }

        self.last_send = Instant::now();
    }

    /// Check how long until next send
    ///
    /// Returns the time remaining until the next packet can be sent.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::timing::TrafficShaper;
    ///
    /// let shaper = TrafficShaper::new(100.0);
    /// let remaining = shaper.time_until_next();
    /// ```
    #[must_use]
    pub fn time_until_next(&self) -> Duration {
        let interval = Duration::from_secs_f64(1.0 / self.target_rate);
        let elapsed = self.last_send.elapsed();

        if elapsed < interval {
            interval - elapsed
        } else {
            Duration::from_secs(0)
        }
    }

    /// Set new target rate
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::timing::TrafficShaper;
    ///
    /// let mut shaper = TrafficShaper::new(100.0);
    /// shaper.set_rate(200.0); // Change to 200 pps
    /// ```
    pub fn set_rate(&mut self, packets_per_second: f64) {
        self.target_rate = packets_per_second;
    }

    /// Get the current target rate
    #[must_use]
    pub const fn rate(&self) -> f64 {
        self.target_rate
    }

    /// Reset the shaper's timing
    ///
    /// Useful when resuming after a pause.
    pub fn reset(&mut self) {
        self.last_send = Instant::now();
    }
}

impl Default for TrafficShaper {
    fn default() -> Self {
        Self::new(100.0) // Default 100 pps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_delay() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Fixed(Duration::from_millis(10)));

        let delay = obfuscator.next_delay();
        assert_eq!(delay, Duration::from_millis(10));

        // Should be consistent
        let delay2 = obfuscator.next_delay();
        assert_eq!(delay2, Duration::from_millis(10));
    }

    #[test]
    fn test_none_delay() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::None);

        let delay = obfuscator.next_delay();
        assert_eq!(delay, Duration::ZERO);
    }

    #[test]
    fn test_uniform_delay() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Uniform {
            min: Duration::from_millis(5),
            max: Duration::from_millis(15),
        });

        for _ in 0..100 {
            let delay = obfuscator.next_delay();
            assert!(delay >= Duration::from_millis(5));
            assert!(delay <= Duration::from_millis(15));
        }
    }

    #[test]
    fn test_uniform_delay_min_equals_max() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Uniform {
            min: Duration::from_millis(10),
            max: Duration::from_millis(10),
        });

        let delay = obfuscator.next_delay();
        assert_eq!(delay, Duration::from_millis(10));
    }

    #[test]
    fn test_uniform_delay_min_greater_than_max() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Uniform {
            min: Duration::from_millis(15),
            max: Duration::from_millis(5),
        });

        // Should return min when min > max
        let delay = obfuscator.next_delay();
        assert_eq!(delay, Duration::from_millis(15));
    }

    #[test]
    fn test_normal_delay() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Normal {
            mean: Duration::from_millis(10),
            stddev: Duration::from_millis(2),
        });

        let mut delays = Vec::new();
        for _ in 0..100 {
            delays.push(obfuscator.next_delay());
        }

        // Should have some variation
        let min_delay = delays.iter().min().unwrap();
        let max_delay = delays.iter().max().unwrap();
        assert!(max_delay > min_delay);

        // All delays should be non-negative
        for delay in delays {
            assert!(delay >= Duration::ZERO);
        }
    }

    #[test]
    fn test_exponential_delay() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Exponential {
            mean: Duration::from_millis(10),
        });

        let mut delays = Vec::new();
        for _ in 0..100 {
            delays.push(obfuscator.next_delay());
        }

        // Exponential distribution should have variation
        let min_delay = delays.iter().min().unwrap();
        let max_delay = delays.iter().max().unwrap();
        assert!(max_delay > min_delay);

        // All delays should be non-negative
        for delay in delays {
            assert!(delay >= Duration::ZERO);
        }
    }

    #[test]
    fn test_traffic_shaper_basic() {
        let shaper = TrafficShaper::new(100.0);
        assert_eq!(shaper.rate(), 100.0);
    }

    #[test]
    fn test_traffic_shaper_set_rate() {
        let mut shaper = TrafficShaper::new(100.0);
        shaper.set_rate(200.0);
        assert_eq!(shaper.rate(), 200.0);
    }

    #[test]
    fn test_traffic_shaper_time_until_next() {
        let shaper = TrafficShaper::new(100.0);
        let time = shaper.time_until_next();

        // Should be close to 10ms (1000ms / 100 pps)
        assert!(time.as_millis() <= 15);
    }

    #[test]
    fn test_traffic_shaper_reset() {
        let mut shaper = TrafficShaper::new(100.0);
        std::thread::sleep(Duration::from_millis(5));

        shaper.reset();

        let time = shaper.time_until_next();
        assert!(time.as_millis() >= 5); // Should be close to full interval
    }

    #[test]
    fn test_traffic_shaper_default() {
        let shaper = TrafficShaper::default();
        assert_eq!(shaper.rate(), 100.0);
    }

    #[test]
    fn test_timing_obfuscator_mode_getter() {
        let obfuscator = TimingObfuscator::new(TimingMode::None);
        assert!(matches!(obfuscator.mode(), TimingMode::None));
    }

    #[test]
    fn test_timing_obfuscator_mode_setter() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::None);
        obfuscator.set_mode(TimingMode::Fixed(Duration::from_millis(5)));

        assert!(matches!(obfuscator.mode(), TimingMode::Fixed(_)));
    }

    #[test]
    fn test_timing_obfuscator_default() {
        let obfuscator = TimingObfuscator::default();
        assert!(matches!(obfuscator.mode(), TimingMode::None));
    }

    #[test]
    fn test_sleep_zero_delay() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::None);
        let start = Instant::now();
        obfuscator.sleep();
        let elapsed = start.elapsed();

        // Should complete almost instantly
        assert!(elapsed < Duration::from_millis(1));
    }

    #[test]
    fn test_sleep_fixed_delay() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Fixed(Duration::from_millis(10)));
        let start = Instant::now();
        obfuscator.sleep();
        let elapsed = start.elapsed();

        // Should sleep for approximately 10ms (with tolerance for CI environments)
        assert!(elapsed >= Duration::from_millis(9));
        assert!(elapsed <= Duration::from_millis(50)); // CI environments can be slow
    }

    #[test]
    fn test_normal_distribution_mean() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Normal {
            mean: Duration::from_millis(100),
            stddev: Duration::from_millis(10),
        });

        let mut total_us = 0u128;
        let samples = 1000;

        for _ in 0..samples {
            total_us += obfuscator.next_delay().as_micros();
        }

        let average_us = total_us / samples;
        let average_ms = average_us / 1000;

        // Average should be close to mean (within 20% due to sampling)
        assert!((80..=120).contains(&average_ms));
    }

    #[test]
    fn test_exponential_distribution_mean() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Exponential {
            mean: Duration::from_millis(50),
        });

        let mut total_us = 0u128;
        let samples = 1000;

        for _ in 0..samples {
            total_us += obfuscator.next_delay().as_micros();
        }

        let average_us = total_us / samples;
        let average_ms = average_us / 1000;

        // Average should be close to mean (within 30% due to exponential variance)
        assert!((35..=65).contains(&average_ms));
    }

    #[test]
    fn test_uniform_distribution_mean() {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Uniform {
            min: Duration::from_millis(40),
            max: Duration::from_millis(60),
        });

        let mut total_us = 0u128;
        let samples = 1000;

        for _ in 0..samples {
            total_us += obfuscator.next_delay().as_micros();
        }

        let average_us = total_us / samples;
        let average_ms = average_us / 1000;

        // Average should be close to midpoint (50ms)
        assert!((45..=55).contains(&average_ms));
    }

    #[test]
    fn test_timing_mode_clone() {
        let mode = TimingMode::Fixed(Duration::from_millis(10));
        let cloned = mode;

        assert!(matches!(cloned, TimingMode::Fixed(_)));
    }

    #[test]
    fn test_all_timing_modes() {
        // Ensure all modes can be constructed
        let modes = [
            TimingMode::None,
            TimingMode::Fixed(Duration::from_millis(10)),
            TimingMode::Uniform {
                min: Duration::from_millis(5),
                max: Duration::from_millis(15),
            },
            TimingMode::Normal {
                mean: Duration::from_millis(10),
                stddev: Duration::from_millis(2),
            },
            TimingMode::Exponential {
                mean: Duration::from_millis(10),
            },
        ];

        for mode in modes {
            let mut obfuscator = TimingObfuscator::new(mode);
            let _delay = obfuscator.next_delay();
        }
    }
}
