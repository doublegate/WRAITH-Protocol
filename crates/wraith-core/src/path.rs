//! Path MTU Discovery for optimal packet sizing.

use std::time::{Duration, Instant};

/// Default initial MTU (common for most networks)
pub const DEFAULT_MTU: u16 = 1280;
/// Maximum MTU to probe
pub const MAX_MTU: u16 = 1500;
/// Minimum MTU (IPv6 minimum)
pub const MIN_MTU: u16 = 1280;

/// Maximum probe attempts per size
const MAX_PROBE_COUNT: u8 = 3;

/// Path MTU Discovery state machine
pub struct PathMtuDiscovery {
    /// Current confirmed MTU
    current_mtu: u16,
    /// Size being probed (None if not probing)
    probe_size: Option<u16>,
    /// Number of probes sent for current size
    probe_count: u8,
    /// Last probe time
    last_probe: Option<Instant>,
    /// Probe interval
    probe_interval: Duration,
}

impl PathMtuDiscovery {
    /// Create a new Path MTU Discovery instance
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_mtu: DEFAULT_MTU,
            probe_size: None,
            probe_count: 0,
            last_probe: None,
            probe_interval: Duration::from_secs(30),
        }
    }

    /// Get current confirmed MTU
    #[must_use]
    pub fn current_mtu(&self) -> u16 {
        self.current_mtu
    }

    /// Start probing for larger MTU
    ///
    /// Returns the size to probe, or `None` if we're already at `MAX_MTU`
    pub fn start_probe(&mut self) -> Option<u16> {
        // Don't start new probe if already probing
        if self.probe_size.is_some() {
            return None;
        }

        // Check if we're already at max
        if self.current_mtu >= MAX_MTU {
            return None;
        }

        // Calculate next probe size (binary search approach)
        let next_size = ((self.current_mtu + MAX_MTU) / 2).max(self.current_mtu + 1);

        self.probe_size = Some(next_size);
        self.probe_count = 0;
        self.last_probe = Some(Instant::now());

        Some(next_size)
    }

    /// Handle probe acknowledgment (success)
    ///
    /// Called when a probe packet is successfully acknowledged
    pub fn probe_acked(&mut self, size: u16) {
        if let Some(probe_size) = self.probe_size
            && size == probe_size
        {
            // Probe succeeded, update confirmed MTU
            self.current_mtu = size;
            self.probe_size = None;
            self.probe_count = 0;
        }
    }

    /// Handle probe timeout/failure
    ///
    /// Called when a probe packet times out or fails
    pub fn probe_failed(&mut self) {
        if let Some(_probe_size) = self.probe_size {
            self.probe_count += 1;

            if self.probe_count >= MAX_PROBE_COUNT {
                // Max retries reached, give up on this size
                self.probe_size = None;
                self.probe_count = 0;
            } else {
                // Retry
                self.last_probe = Some(Instant::now());
            }
        }
    }

    /// Check if we should send a probe
    ///
    /// Returns `true` if:
    /// - We're currently probing and it's time for a retry, or
    /// - Enough time has passed since last probe attempt
    #[must_use]
    pub fn should_probe(&self) -> bool {
        if let Some(last_probe) = self.last_probe {
            // If we're actively probing, check retry interval
            if self.probe_size.is_some() {
                return last_probe.elapsed() >= Duration::from_secs(1);
            }
            // Otherwise, check if it's time for a new probe
            last_probe.elapsed() >= self.probe_interval
        } else {
            // Never probed, should probe now
            true
        }
    }
}

impl Default for PathMtuDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_pmtud_initial_state() {
        let pmtud = PathMtuDiscovery::new();

        assert_eq!(pmtud.current_mtu(), DEFAULT_MTU);
        assert!(pmtud.probe_size.is_none());
        assert_eq!(pmtud.probe_count, 0);
    }

    #[test]
    fn test_pmtud_start_probe() {
        let mut pmtud = PathMtuDiscovery::new();

        let probe_size = pmtud.start_probe();
        assert!(probe_size.is_some());
        assert!(probe_size.unwrap() > DEFAULT_MTU);
        assert!(probe_size.unwrap() <= MAX_MTU);
        assert_eq!(pmtud.probe_size, probe_size);
    }

    #[test]
    fn test_pmtud_cannot_start_multiple_probes() {
        let mut pmtud = PathMtuDiscovery::new();

        let first_probe = pmtud.start_probe();
        assert!(first_probe.is_some());

        let second_probe = pmtud.start_probe();
        assert!(second_probe.is_none());
    }

    #[test]
    fn test_pmtud_probe_acked() {
        let mut pmtud = PathMtuDiscovery::new();

        let probe_size = pmtud.start_probe().unwrap();
        pmtud.probe_acked(probe_size);

        assert_eq!(pmtud.current_mtu(), probe_size);
        assert!(pmtud.probe_size.is_none());
        assert_eq!(pmtud.probe_count, 0);
    }

    #[test]
    fn test_pmtud_probe_failed() {
        let mut pmtud = PathMtuDiscovery::new();

        pmtud.start_probe();
        let initial_count = pmtud.probe_count;

        pmtud.probe_failed();
        assert_eq!(pmtud.probe_count, initial_count + 1);
    }

    #[test]
    fn test_pmtud_probe_max_retries() {
        let mut pmtud = PathMtuDiscovery::new();

        pmtud.start_probe();

        for _ in 0..MAX_PROBE_COUNT {
            pmtud.probe_failed();
        }

        assert!(pmtud.probe_size.is_none());
        assert_eq!(pmtud.probe_count, 0);
    }

    #[test]
    fn test_pmtud_should_probe_initial() {
        let pmtud = PathMtuDiscovery::new();

        // Should probe initially (never probed before)
        assert!(pmtud.should_probe());
    }

    #[test]
    fn test_pmtud_should_probe_retry() {
        let mut pmtud = PathMtuDiscovery::new();

        pmtud.start_probe();

        // Should not probe immediately
        assert!(!pmtud.should_probe());

        // Wait for retry interval
        thread::sleep(Duration::from_millis(1100));

        // Should probe now
        assert!(pmtud.should_probe());
    }

    #[test]
    fn test_pmtud_at_max_mtu() {
        let mut pmtud = PathMtuDiscovery::new();

        // Manually set to max MTU
        pmtud.current_mtu = MAX_MTU;

        let probe_size = pmtud.start_probe();
        assert!(probe_size.is_none());
    }

    #[test]
    fn test_pmtud_binary_search() {
        let mut pmtud = PathMtuDiscovery::new();

        let first_probe = pmtud.start_probe().unwrap();
        pmtud.probe_acked(first_probe);

        let second_probe = pmtud.start_probe().unwrap();

        // Second probe should be between first probe and MAX_MTU
        assert!(second_probe > first_probe);
        assert!(second_probe <= MAX_MTU);
    }

    #[test]
    fn test_pmtud_wrong_size_ack() {
        let mut pmtud = PathMtuDiscovery::new();

        let probe_size = pmtud.start_probe().unwrap();

        // Ack with wrong size (shouldn't update MTU)
        pmtud.probe_acked(probe_size + 100);

        assert_eq!(pmtud.current_mtu(), DEFAULT_MTU);
        assert!(pmtud.probe_size.is_some());
    }

    #[test]
    fn test_pmtud_default() {
        let pmtud1 = PathMtuDiscovery::new();
        let pmtud2 = PathMtuDiscovery::default();

        assert_eq!(pmtud1.current_mtu(), pmtud2.current_mtu());
    }
}
