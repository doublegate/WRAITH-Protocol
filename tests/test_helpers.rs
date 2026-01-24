//! Test helpers for timing-sensitive tests
//!
//! Provides utilities for handling flaky timing tests in CI environments.

use std::time::Duration;

/// Statistical timing validator for flaky tests
///
/// Instead of single-point estimates, this validator runs multiple samples
/// and uses median values to reduce test flakiness in CI environments.
pub struct TimingValidator {
    samples: Vec<Duration>,
    ci_tolerance_multiplier: f64,
}

impl TimingValidator {
    /// Create a new timing validator
    ///
    /// # Arguments
    ///
    /// * `sample_count` - Number of samples to collect (default: 5)
    pub fn new(sample_count: usize) -> Self {
        let ci_tolerance_multiplier = if is_ci_environment() {
            3.0 // 3× more tolerant in CI
        } else {
            1.5 // 1.5× tolerant locally
        };

        Self {
            samples: Vec::with_capacity(sample_count),
            ci_tolerance_multiplier,
        }
    }

    /// Add a timing sample
    pub fn add_sample(&mut self, duration: Duration) {
        self.samples.push(duration);
    }

    /// Get the median of all samples
    pub fn median(&self) -> Option<Duration> {
        if self.samples.is_empty() {
            return None;
        }

        let mut sorted = self.samples.clone();
        sorted.sort();

        let mid = sorted.len() / 2;
        if sorted.len().is_multiple_of(2) {
            Some((sorted[mid - 1] + sorted[mid]) / 2)
        } else {
            Some(sorted[mid])
        }
    }

    /// Get the mean of all samples
    pub fn mean(&self) -> Option<Duration> {
        if self.samples.is_empty() {
            return None;
        }

        let sum: Duration = self.samples.iter().sum();
        Some(sum / self.samples.len() as u32)
    }

    /// Get the CI-adjusted tolerance factor
    pub fn tolerance_multiplier(&self) -> f64 {
        self.ci_tolerance_multiplier
    }

    /// Assert that the median is within tolerance of expected
    ///
    /// # Panics
    ///
    /// Panics if no samples have been added or if the median is outside tolerance.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::time::{Duration, Instant};
    /// use wraith_integration_tests::test_helpers::TimingValidator;
    ///
    /// let mut validator = TimingValidator::new(5);
    /// for _ in 0..5 {
    ///     let start = Instant::now();
    ///     // ... operation ...
    ///     validator.add_sample(start.elapsed());
    /// }
    /// validator.assert_within_tolerance(Duration::from_millis(100), 0.5); // ±50%
    /// ```
    pub fn assert_within_tolerance(&self, expected: Duration, tolerance_ratio: f64) {
        let median = self.median().expect("No samples collected");
        let adjusted_tolerance = tolerance_ratio * self.ci_tolerance_multiplier;

        let lower_bound = expected.mul_f64(1.0 - adjusted_tolerance);
        let upper_bound = expected.mul_f64(1.0 + adjusted_tolerance);

        assert!(
            median >= lower_bound && median <= upper_bound,
            "Median timing {:?} outside tolerance range [{:?}, {:?}] (expected: {:?}, tolerance: {:.1}%, CI-adjusted: {:.1}%)",
            median,
            lower_bound,
            upper_bound,
            expected,
            tolerance_ratio * 100.0,
            adjusted_tolerance * 100.0
        );
    }
}

/// Check if running in a CI environment
///
/// Checks common CI environment variables.
pub fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("CIRCLECI").is_ok()
        || std::env::var("TRAVIS").is_ok()
}

/// Get CI-adjusted timeout duration
///
/// Returns a timeout that's longer in CI environments to account for
/// resource contention and slower machines.
pub fn ci_timeout(base_timeout: Duration) -> Duration {
    if is_ci_environment() {
        base_timeout.mul_f32(3.0)
    } else {
        base_timeout.mul_f32(1.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_validator_median() {
        let mut validator = TimingValidator::new(5);
        validator.add_sample(Duration::from_millis(100));
        validator.add_sample(Duration::from_millis(200));
        validator.add_sample(Duration::from_millis(150));
        validator.add_sample(Duration::from_millis(180));
        validator.add_sample(Duration::from_millis(120));

        let median = validator.median().unwrap();
        assert_eq!(median, Duration::from_millis(150));
    }

    #[test]
    fn test_timing_validator_mean() {
        let mut validator = TimingValidator::new(3);
        validator.add_sample(Duration::from_millis(100));
        validator.add_sample(Duration::from_millis(200));
        validator.add_sample(Duration::from_millis(150));

        let mean = validator.mean().unwrap();
        assert_eq!(mean, Duration::from_millis(150));
    }

    #[test]
    fn test_ci_timeout() {
        let base = Duration::from_secs(10);
        let adjusted = ci_timeout(base);

        if is_ci_environment() {
            assert_eq!(adjusted, Duration::from_secs(30));
        } else {
            assert_eq!(adjusted, Duration::from_secs(15));
        }
    }
}
