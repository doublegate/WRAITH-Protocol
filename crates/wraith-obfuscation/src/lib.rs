//! # WRAITH Obfuscation
//!
//! Traffic obfuscation layer for the WRAITH protocol.
//!
//! This crate provides:
//! - Packet padding to fixed size classes
//! - Timing obfuscation with jitter
//! - Cover traffic generation
//! - Protocol mimicry (HTTPS, WebSocket, DoH)

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod cover;
pub mod padding;
pub mod timing;

/// Padding mode for traffic analysis resistance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddingMode {
    /// Minimal padding for maximum performance
    Performance,
    /// Random padding class selection
    Privacy,
    /// Match typical HTTPS traffic patterns
    Stealth,
}

/// Timing mode for inter-packet delay
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingMode {
    /// No delay, maximum throughput
    LowLatency,
    /// Moderate jitter
    Moderate,
    /// Match HTTPS timing patterns
    HighPrivacy,
}
