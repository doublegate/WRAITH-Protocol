//! BBR-inspired congestion control.
//!
//! Implements a BBRv2-inspired congestion control algorithm optimized
//! for high-throughput, low-latency file transfers.

use std::time::{Duration, Instant};

/// BBR congestion control state
#[allow(dead_code)]
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
}

/// BBR algorithm phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbrPhase {
    /// Exponential bandwidth probing
    Startup,
    /// Reduce in-flight to drain queue
    Drain,
    /// Steady state with periodic probing
    ProbeBw,
    /// Periodic RTT measurement
    ProbeRtt,
}

impl BbrState {
    /// Create new BBR state
    pub fn new() -> Self {
        Self {
            btl_bw: 0,
            min_rtt: Duration::from_millis(100), // Initial estimate
            pacing_gain: 2.89,                   // Startup gain
            cwnd_gain: 2.0,
            bdp: 0,
            phase: BbrPhase::Startup,
            round_count: 0,
            state_start: Instant::now(),
        }
    }

    /// Get current pacing rate (bytes/sec)
    pub fn pacing_rate(&self) -> u64 {
        (self.btl_bw as f64 * self.pacing_gain) as u64
    }

    /// Get current congestion window
    pub fn cwnd(&self) -> u64 {
        (self.bdp as f64 * self.cwnd_gain) as u64
    }

    /// Check if we can send more data
    pub fn can_send(&self, _bytes: usize) -> bool {
        // TODO: Implement proper in-flight tracking
        true
    }

    /// Get current phase
    pub fn phase(&self) -> BbrPhase {
        self.phase
    }
}

impl Default for BbrState {
    fn default() -> Self {
        Self::new()
    }
}
