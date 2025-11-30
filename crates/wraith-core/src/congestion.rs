//! `BBR`-inspired congestion control.
//!
//! Implements a `BBRv2`-inspired congestion control algorithm optimized
//! for high-throughput, low-latency file transfers.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Maximum number of bandwidth samples to keep
const BW_WINDOW_SIZE: usize = 10;

/// Maximum number of RTT samples to keep
const RTT_WINDOW_SIZE: usize = 10;

/// Time to stay in `ProbeRtt` phase (10ms)
const PROBE_RTT_DURATION: Duration = Duration::from_millis(10);

/// Interval between `ProbeRtt` phases (10 seconds)
const PROBE_RTT_INTERVAL: Duration = Duration::from_secs(10);

/// Minimum inflight during `ProbeRtt` (4 packets worth)
const PROBE_RTT_MIN_INFLIGHT: u64 = 4 * 1_500;

/// `BBR` congestion control state
pub struct BbrState {
    /// Estimated bottleneck bandwidth (bytes/sec)
    btl_bw: u64,
    /// Minimum observed RTT
    min_rtt: Duration,
    /// Current pacing gain
    pacing_gain: f64,
    /// Current cwnd gain
    cwnd_gain: f64,
    /// Bandwidth-Delay Product
    bdp: u64,
    /// Current phase
    phase: BbrPhase,
    /// Round-trip counter
    round_count: u64,
    /// Time when current state entered
    state_start: Instant,
    /// Bandwidth samples (bytes delivered, time interval)
    bw_samples: VecDeque<(u64, Duration)>,
    /// RTT samples
    rtt_samples: VecDeque<Duration>,
    /// Bytes in flight
    bytes_in_flight: u64,
    /// Bytes delivered (for bandwidth estimation)
    bytes_delivered: u64,
    /// Time of last delivery
    last_delivery_time: Instant,
    /// Last time we entered `ProbeRtt`
    last_probe_rtt: Instant,
    /// Time in current `ProbeRtt`
    probe_rtt_start: Option<Instant>,
    /// Cycle index for `ProbeBw` (0-7)
    probe_bw_cycle_idx: usize,
    /// Rounds without bandwidth growth (for `Startup` exit)
    rounds_without_growth: u64,
    /// Prior `btl_bw` (for growth detection)
    prior_btl_bw: u64,
}

/// `BBR` algorithm phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbrPhase {
    /// Exponential bandwidth probing
    Startup,
    /// Reduce in-flight to drain queue
    Drain,
    /// Steady state with periodic probing
    ProbeBw,
    /// Periodic `RTT` measurement
    ProbeRtt,
}

impl BbrState {
    /// Create new `BBR` state
    #[must_use]
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            btl_bw: 0,
            min_rtt: Duration::from_millis(100), // Initial estimate
            pacing_gain: 2.89,                   // Startup gain (2/ln(2))
            cwnd_gain: 2.0,
            bdp: 0,
            phase: BbrPhase::Startup,
            round_count: 0,
            state_start: now,
            bw_samples: VecDeque::with_capacity(BW_WINDOW_SIZE),
            rtt_samples: VecDeque::with_capacity(RTT_WINDOW_SIZE),
            bytes_in_flight: 0,
            bytes_delivered: 0,
            last_delivery_time: now,
            last_probe_rtt: now,
            probe_rtt_start: None,
            probe_bw_cycle_idx: 0,
            rounds_without_growth: 0,
            prior_btl_bw: 0,
        }
    }

    /// Update `RTT` estimate with new sample
    pub fn update_rtt(&mut self, rtt_sample: Duration) {
        // Add sample to window
        self.rtt_samples.push_back(rtt_sample);
        if self.rtt_samples.len() > RTT_WINDOW_SIZE {
            self.rtt_samples.pop_front();
        }

        // Update min_rtt if we have a new minimum
        if let Some(&min) = self.rtt_samples.iter().min() {
            self.min_rtt = min;
        }
    }

    /// Update bandwidth estimate
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn update_bandwidth(&mut self, bytes_delivered: u64, interval: Duration) {
        if interval.as_secs_f64() > 0.0 {
            // Note: precision loss is acceptable for bandwidth estimation
            let bw = (bytes_delivered as f64 / interval.as_secs_f64()) as u64;

            // Add sample to window
            self.bw_samples.push_back((bw, interval));
            if self.bw_samples.len() > BW_WINDOW_SIZE {
                self.bw_samples.pop_front();
            }

            // Update `btl_bw` to max of window
            if let Some(&(max_bw, _)) = self.bw_samples.iter().max_by_key(|(bw, _)| bw) {
                self.btl_bw = max_bw;
            }

            // Update `BDP`
            self.bdp = (self.btl_bw as f64 * self.min_rtt.as_secs_f64()) as u64;
        }
    }

    /// Get current pacing rate (bytes/sec)
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn pacing_rate(&self) -> u64 {
        if self.btl_bw == 0 {
            // Initial rate: 10 Mbps
            return 10_000_000 / 8;
        }
        // Note: precision loss is acceptable for pacing rate calculation
        (self.btl_bw as f64 * self.pacing_gain) as u64
    }

    /// Get current congestion window
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn cwnd(&self) -> u64 {
        if self.bdp == 0 {
            // Initial window: 10 packets
            return 10 * 1_500;
        }

        if self.phase == BbrPhase::ProbeRtt {
            // Minimum inflight during `ProbeRtt`
            PROBE_RTT_MIN_INFLIGHT
        } else {
            // Note: precision loss is acceptable for window calculation
            let cwnd = (self.bdp as f64 * self.cwnd_gain) as u64;
            // Minimum of 4 packets
            cwnd.max(4 * 1_500)
        }
    }

    /// Check if we can send more data
    #[must_use]
    pub fn can_send(&self, bytes: u64) -> bool {
        self.bytes_in_flight + bytes <= self.cwnd()
    }

    /// Get current phase
    #[must_use]
    pub fn phase(&self) -> BbrPhase {
        self.phase
    }

    /// Called when a packet is sent
    pub fn on_packet_sent(&mut self, bytes: u64) {
        self.bytes_in_flight += bytes;
    }

    /// Called when a packet is acknowledged
    pub fn on_packet_acked(&mut self, bytes: u64, rtt: Duration) {
        self.bytes_in_flight = self.bytes_in_flight.saturating_sub(bytes);
        self.bytes_delivered += bytes;

        // Update RTT estimate
        self.update_rtt(rtt);

        // Update bandwidth estimate
        let now = Instant::now();
        let interval = now.duration_since(self.last_delivery_time);
        if interval.as_millis() > 0 {
            self.update_bandwidth(bytes, interval);
            self.last_delivery_time = now;
        }

        // Update state machine
        self.update();
    }

    /// Called when a packet is lost
    pub fn on_packet_lost(&mut self, bytes: u64) {
        self.bytes_in_flight = self.bytes_in_flight.saturating_sub(bytes);
    }

    /// Update `BBR` state machine
    pub fn update(&mut self) {
        let now = Instant::now();

        // Check for state transitions
        match self.phase {
            BbrPhase::Startup => {
                // Exit `Startup` if bandwidth plateaus
                if self.should_exit_startup() {
                    self.enter_drain();
                }
            }
            BbrPhase::Drain => {
                // Exit `Drain` when inflight <= `BDP`
                if self.bytes_in_flight <= self.bdp {
                    self.enter_probe_bw();
                }
            }
            BbrPhase::ProbeBw => {
                // Check if we should enter `ProbeRtt`
                if now.duration_since(self.last_probe_rtt) >= PROBE_RTT_INTERVAL {
                    self.enter_probe_rtt();
                } else {
                    // Cycle through pacing gains
                    self.advance_probe_bw_cycle();
                }
            }
            BbrPhase::ProbeRtt => {
                // Stay in `ProbeRtt` for at least `PROBE_RTT_DURATION`
                if let Some(start) = self.probe_rtt_start {
                    if now.duration_since(start) >= PROBE_RTT_DURATION {
                        self.exit_probe_rtt();
                    }
                }
            }
        }
    }

    /// Check if we should exit `Startup` phase
    #[allow(clippy::cast_precision_loss)]
    fn should_exit_startup(&mut self) -> bool {
        // Exit if bandwidth hasn't grown for 3 rounds
        const GROWTH_THRESHOLD: f64 = 1.25; // 25% growth

        if self.btl_bw > 0 && self.prior_btl_bw > 0 {
            // Note: precision loss is acceptable for growth detection
            let growth = self.btl_bw as f64 / self.prior_btl_bw as f64;
            if growth < GROWTH_THRESHOLD {
                self.rounds_without_growth += 1;
            } else {
                self.rounds_without_growth = 0;
            }
        }

        self.prior_btl_bw = self.btl_bw;

        self.rounds_without_growth >= 3
    }

    /// Enter `Drain` phase
    fn enter_drain(&mut self) {
        self.phase = BbrPhase::Drain;
        self.pacing_gain = 1.0 / 2.89; // Inverse of `Startup` gain
        self.cwnd_gain = 2.0;
        self.state_start = Instant::now();
    }

    /// Enter `ProbeBw` phase
    fn enter_probe_bw(&mut self) {
        self.phase = BbrPhase::ProbeBw;
        self.probe_bw_cycle_idx = 0;
        self.set_probe_bw_gains();
        self.state_start = Instant::now();
    }

    /// Enter `ProbeRtt` phase
    fn enter_probe_rtt(&mut self) {
        self.phase = BbrPhase::ProbeRtt;
        self.pacing_gain = 1.0;
        self.cwnd_gain = 1.0;
        self.probe_rtt_start = Some(Instant::now());
        self.last_probe_rtt = Instant::now();
        self.state_start = Instant::now();
    }

    /// Exit `ProbeRtt` phase
    fn exit_probe_rtt(&mut self) {
        self.probe_rtt_start = None;
        // Return to `ProbeBw`
        self.enter_probe_bw();
    }

    /// Advance `ProbeBw` cycle
    fn advance_probe_bw_cycle(&mut self) {
        self.round_count += 1;
        if self.round_count % 8 == 0 {
            self.probe_bw_cycle_idx = (self.probe_bw_cycle_idx + 1) % 8;
            self.set_probe_bw_gains();
        }
    }

    /// Set pacing/cwnd gains for current `ProbeBw` cycle
    fn set_probe_bw_gains(&mut self) {
        // `ProbeBw` cycle: [1.25, 0.75, 1, 1, 1, 1, 1, 1]
        const PROBE_BW_GAINS: [f64; 8] = [1.25, 0.75, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];

        self.pacing_gain = PROBE_BW_GAINS[self.probe_bw_cycle_idx];
        self.cwnd_gain = 2.0;
    }

    /// Get minimum `RTT`
    #[must_use]
    pub fn min_rtt(&self) -> Duration {
        self.min_rtt
    }

    /// Get bottleneck bandwidth
    #[must_use]
    pub fn btl_bw(&self) -> u64 {
        self.btl_bw
    }

    /// Get `BDP`
    #[must_use]
    pub fn bdp(&self) -> u64 {
        self.bdp
    }

    /// Get bytes in flight
    #[must_use]
    pub fn bytes_in_flight(&self) -> u64 {
        self.bytes_in_flight
    }
}

impl Default for BbrState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_bbr_initial_state() {
        let bbr = BbrState::new();

        assert_eq!(bbr.phase(), BbrPhase::Startup);
        assert_eq!(bbr.bytes_in_flight(), 0);
        assert_eq!(bbr.btl_bw(), 0);
        assert!(bbr.pacing_rate() > 0); // Should have initial rate
        assert!(bbr.cwnd() > 0); // Should have initial cwnd
    }

    #[test]
    fn test_bbr_rtt_update() {
        let mut bbr = BbrState::new();

        let rtt1 = Duration::from_millis(50);
        let rtt2 = Duration::from_millis(30);
        let rtt3 = Duration::from_millis(40);

        bbr.update_rtt(rtt1);
        bbr.update_rtt(rtt2);
        bbr.update_rtt(rtt3);

        // Min RTT should be the smallest sample
        assert_eq!(bbr.min_rtt(), rtt2);
    }

    #[test]
    fn test_bbr_rtt_window_limit() {
        let mut bbr = BbrState::new();

        // Add more samples than window size
        for i in 1..=15 {
            bbr.update_rtt(Duration::from_millis(i * 10));
        }

        // Should only keep last RTT_WINDOW_SIZE samples
        assert!(bbr.rtt_samples.len() <= RTT_WINDOW_SIZE);
    }

    #[test]
    fn test_bbr_bandwidth_estimation() {
        let mut bbr = BbrState::new();

        // Simulate 1 MB delivered in 1 second = 1 MB/s
        let bytes = 1_000_000;
        let interval = Duration::from_secs(1);

        bbr.update_bandwidth(bytes, interval);

        assert_eq!(bbr.btl_bw(), bytes);
        assert!(bbr.bdp() > 0); // BDP should be calculated
    }

    #[test]
    fn test_bbr_bandwidth_window_max() {
        let mut bbr = BbrState::new();

        // Add multiple bandwidth samples
        bbr.update_bandwidth(1_000_000, Duration::from_secs(1)); // 1 MB/s
        bbr.update_bandwidth(2_000_000, Duration::from_secs(1)); // 2 MB/s (max)
        bbr.update_bandwidth(1_500_000, Duration::from_secs(1)); // 1.5 MB/s

        // Should use the maximum
        assert_eq!(bbr.btl_bw(), 2_000_000);
    }

    #[test]
    fn test_bbr_bdp_calculation() {
        let mut bbr = BbrState::new();

        bbr.update_rtt(Duration::from_millis(100));
        bbr.update_bandwidth(10_000_000, Duration::from_secs(1)); // 10 MB/s

        // BDP = bandwidth × RTT
        // 10 MB/s × 0.1s = 1 MB
        let expected_bdp = (10_000_000.0 * 0.1) as u64;
        assert_eq!(bbr.bdp(), expected_bdp);
    }

    #[test]
    fn test_bbr_packet_sent() {
        let mut bbr = BbrState::new();

        assert_eq!(bbr.bytes_in_flight(), 0);

        bbr.on_packet_sent(1500);
        assert_eq!(bbr.bytes_in_flight(), 1500);

        bbr.on_packet_sent(1500);
        assert_eq!(bbr.bytes_in_flight(), 3000);
    }

    #[test]
    fn test_bbr_packet_acked() {
        let mut bbr = BbrState::new();

        bbr.on_packet_sent(1500);
        assert_eq!(bbr.bytes_in_flight(), 1500);

        thread::sleep(Duration::from_millis(10));

        bbr.on_packet_acked(1500, Duration::from_millis(50));
        assert_eq!(bbr.bytes_in_flight(), 0);
        assert_eq!(bbr.min_rtt(), Duration::from_millis(50));
    }

    #[test]
    fn test_bbr_packet_lost() {
        let mut bbr = BbrState::new();

        bbr.on_packet_sent(3000);
        assert_eq!(bbr.bytes_in_flight(), 3000);

        bbr.on_packet_lost(1500);
        assert_eq!(bbr.bytes_in_flight(), 1500);
    }

    #[test]
    fn test_bbr_can_send() {
        let mut bbr = BbrState::new();

        // Should be able to send within cwnd
        let cwnd = bbr.cwnd();
        assert!(bbr.can_send(cwnd / 2));

        // Fill up the window
        bbr.on_packet_sent(cwnd);
        assert!(!bbr.can_send(1));
    }

    #[test]
    fn test_bbr_startup_phase() {
        let bbr = BbrState::new();

        assert_eq!(bbr.phase(), BbrPhase::Startup);
        // Startup pacing gain should be 2.89
        assert!((bbr.pacing_gain - 2.89).abs() < 0.01);
    }

    #[test]
    fn test_bbr_startup_exit_on_plateau() {
        let mut bbr = BbrState::new();

        // Simulate bandwidth plateau
        bbr.update_bandwidth(1_000_000, Duration::from_secs(1));
        bbr.update(); // Round 1

        bbr.update_bandwidth(1_100_000, Duration::from_secs(1)); // 10% growth
        bbr.update(); // Round 2

        bbr.update_bandwidth(1_150_000, Duration::from_secs(1)); // <25% growth
        bbr.update(); // Round 3

        bbr.update_bandwidth(1_180_000, Duration::from_secs(1)); // <25% growth
        bbr.update(); // Round 4 - should exit Startup

        assert_eq!(bbr.phase(), BbrPhase::Drain);
    }

    #[test]
    fn test_bbr_drain_phase() {
        let mut bbr = BbrState::new();

        // Force into Drain phase
        bbr.enter_drain();

        assert_eq!(bbr.phase(), BbrPhase::Drain);
        // Drain pacing gain should be 1/2.89
        assert!((bbr.pacing_gain - 1.0 / 2.89).abs() < 0.01);
    }

    #[test]
    fn test_bbr_drain_to_probe_bw() {
        let mut bbr = BbrState::new();

        // Set up conditions for Drain
        bbr.update_bandwidth(10_000_000, Duration::from_secs(1));
        bbr.update_rtt(Duration::from_millis(100));
        bbr.enter_drain();

        // Bytes in flight > BDP
        bbr.on_packet_sent(bbr.bdp() * 2);

        bbr.update();
        assert_eq!(bbr.phase(), BbrPhase::Drain);

        // Reduce bytes in flight below BDP
        bbr.on_packet_lost(bbr.bdp());
        bbr.update();

        assert_eq!(bbr.phase(), BbrPhase::ProbeBw);
    }

    #[test]
    fn test_bbr_probe_bw_phase() {
        let mut bbr = BbrState::new();

        bbr.enter_probe_bw();

        assert_eq!(bbr.phase(), BbrPhase::ProbeBw);
        assert_eq!(bbr.probe_bw_cycle_idx, 0);
    }

    #[test]
    fn test_bbr_probe_bw_cycle() {
        let mut bbr = BbrState::new();

        bbr.enter_probe_bw();

        let initial_gain = bbr.pacing_gain;

        // Initial gain should be 1.25 (first element of PROBE_BW_GAINS)
        assert_eq!(initial_gain, 1.25);

        // Advance through cycles
        for _ in 0..8 {
            bbr.advance_probe_bw_cycle();
        }

        // Should cycle to index 1
        assert_eq!(bbr.probe_bw_cycle_idx, 1);

        // Gain should have changed to 0.75 (second element of PROBE_BW_GAINS)
        assert_eq!(bbr.pacing_gain, 0.75);
    }

    #[test]
    fn test_bbr_probe_rtt_entry() {
        let mut bbr = BbrState::new();

        bbr.enter_probe_rtt();

        assert_eq!(bbr.phase(), BbrPhase::ProbeRtt);
        assert!(bbr.probe_rtt_start.is_some());
        assert_eq!(bbr.pacing_gain, 1.0);
        assert_eq!(bbr.cwnd_gain, 1.0);
    }

    #[test]
    fn test_bbr_probe_rtt_cwnd() {
        let mut bbr = BbrState::new();

        // Set up BDP
        bbr.update_bandwidth(10_000_000, Duration::from_secs(1));
        bbr.update_rtt(Duration::from_millis(100));

        let normal_cwnd = bbr.cwnd();

        bbr.enter_probe_rtt();

        // ProbeRtt cwnd should be minimal
        assert_eq!(bbr.cwnd(), PROBE_RTT_MIN_INFLIGHT);
        assert!(bbr.cwnd() < normal_cwnd);
    }

    #[test]
    fn test_bbr_probe_rtt_exit() {
        let mut bbr = BbrState::new();

        bbr.enter_probe_rtt();
        assert_eq!(bbr.phase(), BbrPhase::ProbeRtt);

        // Manually exit
        bbr.exit_probe_rtt();

        assert_eq!(bbr.phase(), BbrPhase::ProbeBw);
        assert!(bbr.probe_rtt_start.is_none());
    }

    #[test]
    fn test_bbr_pacing_rate_initial() {
        let bbr = BbrState::new();

        // Should have a default pacing rate even with no bandwidth estimate
        let rate = bbr.pacing_rate();
        assert!(rate > 0);
        assert_eq!(rate, 10_000_000 / 8); // 10 Mbps initial
    }

    #[test]
    fn test_bbr_pacing_rate_with_bandwidth() {
        let mut bbr = BbrState::new();

        bbr.update_bandwidth(5_000_000, Duration::from_secs(1)); // 5 MB/s

        let rate = bbr.pacing_rate();
        let expected = (5_000_000.0 * 2.89) as u64; // Startup gain

        assert_eq!(rate, expected);
    }

    #[test]
    fn test_bbr_cwnd_initial() {
        let bbr = BbrState::new();

        // Should have initial cwnd
        let cwnd = bbr.cwnd();
        assert_eq!(cwnd, 10 * 1500); // 10 packets
    }

    #[test]
    fn test_bbr_cwnd_with_bdp() {
        let mut bbr = BbrState::new();

        bbr.update_bandwidth(10_000_000, Duration::from_secs(1));
        bbr.update_rtt(Duration::from_millis(100));

        let cwnd = bbr.cwnd();
        let expected_bdp = (10_000_000.0 * 0.1) as u64;
        let expected_cwnd = (expected_bdp as f64 * 2.0) as u64; // cwnd_gain = 2.0

        assert_eq!(cwnd, expected_cwnd);
    }

    #[test]
    fn test_bbr_inflight_tracking() {
        let mut bbr = BbrState::new();

        assert_eq!(bbr.bytes_in_flight(), 0);

        bbr.on_packet_sent(1500);
        bbr.on_packet_sent(1500);
        bbr.on_packet_sent(1500);
        assert_eq!(bbr.bytes_in_flight(), 4500);

        bbr.on_packet_acked(1500, Duration::from_millis(50));
        assert_eq!(bbr.bytes_in_flight(), 3000);

        bbr.on_packet_lost(1500);
        assert_eq!(bbr.bytes_in_flight(), 1500);
    }

    #[test]
    fn test_bbr_state_transitions() {
        let mut bbr = BbrState::new();

        // Start in Startup
        assert_eq!(bbr.phase(), BbrPhase::Startup);

        // Transition to Drain
        bbr.enter_drain();
        assert_eq!(bbr.phase(), BbrPhase::Drain);

        // Transition to ProbeBw
        bbr.enter_probe_bw();
        assert_eq!(bbr.phase(), BbrPhase::ProbeBw);

        // Transition to ProbeRtt
        bbr.enter_probe_rtt();
        assert_eq!(bbr.phase(), BbrPhase::ProbeRtt);

        // Back to ProbeBw
        bbr.exit_probe_rtt();
        assert_eq!(bbr.phase(), BbrPhase::ProbeBw);
    }

    #[test]
    fn test_bbr_accessors() {
        let mut bbr = BbrState::new();

        bbr.update_bandwidth(5_000_000, Duration::from_secs(1));
        bbr.update_rtt(Duration::from_millis(50));

        assert_eq!(bbr.btl_bw(), 5_000_000);
        assert_eq!(bbr.min_rtt(), Duration::from_millis(50));
        assert!(bbr.bdp() > 0);
        assert_eq!(bbr.bytes_in_flight(), 0);
    }

    // ========================================================================
    // Additional Phase Transition Tests (Tier 4 - Technical Debt Remediation)
    // ========================================================================

    #[test]
    fn test_bbr_complete_phase_cycle() {
        // Test a complete cycle through all phases
        let mut bbr = BbrState::new();

        // 1. Start in Startup
        assert_eq!(bbr.phase(), BbrPhase::Startup);

        // 2. Transition to Drain
        bbr.enter_drain();
        assert_eq!(bbr.phase(), BbrPhase::Drain);
        // Verify Drain pacing gain is inverse of Startup
        assert!((bbr.pacing_gain - (1.0 / 2.89)).abs() < 0.01);

        // 3. Transition to ProbeBw
        bbr.enter_probe_bw();
        assert_eq!(bbr.phase(), BbrPhase::ProbeBw);
        // First cycle index should be 0 with gain 1.25
        assert_eq!(bbr.pacing_gain, 1.25);

        // 4. Transition to ProbeRtt
        bbr.enter_probe_rtt();
        assert_eq!(bbr.phase(), BbrPhase::ProbeRtt);
        assert_eq!(bbr.pacing_gain, 1.0);
        assert_eq!(bbr.cwnd_gain, 1.0);

        // 5. Exit back to ProbeBw
        bbr.exit_probe_rtt();
        assert_eq!(bbr.phase(), BbrPhase::ProbeBw);
    }

    #[test]
    fn test_bbr_startup_continuous_growth() {
        // Verify Startup doesn't exit when bandwidth keeps growing
        let mut bbr = BbrState::new();

        // Simulate continuous bandwidth growth (>25% per round)
        let mut bandwidth = 1_000_000u64;
        for _ in 0..10 {
            bandwidth = (bandwidth as f64 * 1.3) as u64; // 30% growth
            bbr.update_bandwidth(bandwidth, Duration::from_secs(1));
            bbr.update();
        }

        // Should still be in Startup
        assert_eq!(bbr.phase(), BbrPhase::Startup);
    }

    #[test]
    fn test_bbr_bandwidth_estimation_accuracy() {
        let mut bbr = BbrState::new();

        // Simulate consistent bandwidth
        let target_bw = 100_000_000u64; // 100 MB/s
        for _ in 0..5 {
            bbr.update_bandwidth(target_bw, Duration::from_secs(1));
        }

        // Bandwidth estimate should match
        assert_eq!(bbr.btl_bw(), target_bw);
    }

    #[test]
    fn test_bbr_rtt_measurement_accuracy() {
        let mut bbr = BbrState::new();

        // Add RTT samples with some variance
        let rtts = [
            Duration::from_millis(50),
            Duration::from_millis(52),
            Duration::from_millis(48),
            Duration::from_millis(55),
            Duration::from_millis(45),
        ];

        for rtt in &rtts {
            bbr.update_rtt(*rtt);
        }

        // min_rtt should be the minimum (45ms)
        assert_eq!(bbr.min_rtt(), Duration::from_millis(45));
    }

    #[test]
    fn test_bbr_probe_bw_full_cycle() {
        let mut bbr = BbrState::new();
        bbr.enter_probe_bw();

        // Expected gains for full cycle: [1.25, 0.75, 1, 1, 1, 1, 1, 1]
        let expected_gains = [1.25, 0.75, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];

        for (idx, expected_gain) in expected_gains.iter().enumerate() {
            assert!(
                (bbr.pacing_gain - expected_gain).abs() < 0.01,
                "Cycle {} expected gain {}, got {}",
                idx,
                expected_gain,
                bbr.pacing_gain
            );

            // Advance 8 rounds to move to next cycle
            for _ in 0..8 {
                bbr.advance_probe_bw_cycle();
            }
        }
    }

    #[test]
    fn test_bbr_inflight_never_negative() {
        let mut bbr = BbrState::new();

        // Send some data
        bbr.on_packet_sent(1500);
        bbr.on_packet_sent(1500);

        // Lose more than in flight (should saturate at 0)
        bbr.on_packet_lost(5000);

        assert_eq!(bbr.bytes_in_flight(), 0);
    }

    #[test]
    fn test_bbr_cwnd_minimum() {
        let bbr = BbrState::new();

        // With zero BDP, cwnd should be at minimum
        assert!(bbr.cwnd() >= 4 * 1500); // At least 4 packets
    }

    #[test]
    fn test_bbr_default_impl() {
        let bbr1 = BbrState::new();
        let bbr2 = BbrState::default();

        // Both should start in same state
        assert_eq!(bbr1.phase(), bbr2.phase());
        assert_eq!(bbr1.btl_bw(), bbr2.btl_bw());
        assert_eq!(bbr1.bytes_in_flight(), bbr2.bytes_in_flight());
    }
}
