//! FFI-safe type definitions

use std::time::Duration;

/// Node ID (32 bytes - Ed25519 public key)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WraithNodeId {
    pub bytes: [u8; 32],
}

/// Session ID (32 bytes - unique identifier)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WraithSessionId {
    pub bytes: [u8; 32],
}

/// Transfer ID (32 bytes - unique identifier)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WraithTransferId {
    pub bytes: [u8; 32],
}

/// Connection statistics
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WraithConnectionStats {
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Round-trip time in microseconds
    pub rtt_us: u64,
    /// Packet loss rate (0.0 to 1.0)
    pub loss_rate: f32,
}

/// Transfer progress information
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WraithTransferProgress {
    /// Total bytes
    pub total_bytes: u64,
    /// Bytes transferred
    pub transferred_bytes: u64,
    /// Progress percentage (0.0 to 1.0)
    pub progress: f32,
    /// Estimated time remaining in seconds (0 if unknown)
    pub eta_seconds: u64,
    /// Current transfer rate in bytes/second
    pub rate_bytes_per_sec: u64,
    /// Transfer is complete
    pub is_complete: bool,
}

/// Transfer status
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WraithTransferStatus {
    /// Transfer is initializing
    Initializing = 0,
    /// Transfer is in progress
    InProgress = 1,
    /// Transfer completed successfully
    Completed = 2,
    /// Transfer failed
    Failed = 3,
    /// Transfer was cancelled
    Cancelled = 4,
}

/// Padding mode for obfuscation
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WraithPaddingMode {
    /// No padding
    None = 0,
    /// Pad to nearest power of two
    PowerOfTwo = 1,
    /// Pad to size class buckets
    SizeClasses = 2,
    /// Constant rate padding
    ConstantRate = 3,
    /// Statistical padding
    Statistical = 4,
}

/// Timing mode for obfuscation
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WraithTimingMode {
    /// No timing obfuscation
    None = 0,
    /// Fixed delay (10ms)
    Fixed = 1,
    /// Uniform random delay (5-50ms)
    Uniform = 2,
    /// Normal distribution delay (mean 20ms, stddev 10ms)
    Normal = 3,
    /// Exponential distribution delay (mean 20ms)
    Exponential = 4,
}

/// Protocol mimicry mode
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WraithMimicryMode {
    /// No protocol mimicry
    None = 0,
    /// Mimic TLS 1.3
    Tls = 1,
    /// Mimic WebSocket
    WebSocket = 2,
    /// Mimic DNS over HTTPS
    Doh = 3,
}

/// Log level
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WraithLogLevel {
    /// Trace level logging
    Trace = 0,
    /// Debug level logging
    Debug = 1,
    /// Info level logging
    Info = 2,
    /// Warning level logging
    Warn = 3,
    /// Error level logging
    Error = 4,
}

// Conversions to wraith_core::node types (used by NodeConfig)
impl From<WraithPaddingMode> for wraith_core::node::PaddingMode {
    fn from(mode: WraithPaddingMode) -> Self {
        match mode {
            WraithPaddingMode::None => wraith_core::node::PaddingMode::None,
            WraithPaddingMode::PowerOfTwo => wraith_core::node::PaddingMode::PowerOfTwo,
            WraithPaddingMode::SizeClasses => wraith_core::node::PaddingMode::SizeClasses,
            WraithPaddingMode::ConstantRate => wraith_core::node::PaddingMode::ConstantRate,
            WraithPaddingMode::Statistical => wraith_core::node::PaddingMode::Statistical,
        }
    }
}

impl From<WraithTimingMode> for wraith_core::node::TimingMode {
    fn from(mode: WraithTimingMode) -> Self {
        match mode {
            WraithTimingMode::None => wraith_core::node::TimingMode::None,
            WraithTimingMode::Fixed => {
                wraith_core::node::TimingMode::Fixed(Duration::from_millis(10))
            }
            WraithTimingMode::Uniform => wraith_core::node::TimingMode::Uniform {
                min: Duration::from_millis(5),
                max: Duration::from_millis(50),
            },
            WraithTimingMode::Normal => wraith_core::node::TimingMode::Normal {
                mean: Duration::from_millis(20),
                stddev: Duration::from_millis(10),
            },
            WraithTimingMode::Exponential => wraith_core::node::TimingMode::Exponential {
                mean: Duration::from_millis(20),
            },
        }
    }
}

impl From<WraithMimicryMode> for wraith_core::node::MimicryMode {
    fn from(mode: WraithMimicryMode) -> Self {
        match mode {
            WraithMimicryMode::None => wraith_core::node::MimicryMode::None,
            WraithMimicryMode::Tls => wraith_core::node::MimicryMode::Tls,
            WraithMimicryMode::WebSocket => wraith_core::node::MimicryMode::WebSocket,
            WraithMimicryMode::Doh => wraith_core::node::MimicryMode::DoH,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_size() {
        assert_eq!(std::mem::size_of::<WraithNodeId>(), 32);
    }

    #[test]
    fn test_session_id_size() {
        assert_eq!(std::mem::size_of::<WraithSessionId>(), 32);
    }

    #[test]
    fn test_transfer_id_size() {
        assert_eq!(std::mem::size_of::<WraithTransferId>(), 32);
    }

    #[test]
    fn test_padding_mode_conversion() {
        let mode = WraithPaddingMode::PowerOfTwo;
        let core_mode: wraith_core::node::PaddingMode = mode.into();
        assert!(matches!(
            core_mode,
            wraith_core::node::PaddingMode::PowerOfTwo
        ));
    }

    #[test]
    fn test_timing_mode_conversion() {
        let mode = WraithTimingMode::Uniform;
        let core_mode: wraith_core::node::TimingMode = mode.into();
        assert!(matches!(
            core_mode,
            wraith_core::node::TimingMode::Uniform { .. }
        ));
    }
}
