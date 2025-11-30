//! # WRAITH Transport
//!
//! Network transport layer for the WRAITH protocol.
//!
//! This crate provides:
//! - AF_XDP socket management for zero-copy packet I/O
//! - io_uring integration for async file operations
//! - UDP socket fallback for non-Linux systems
//! - Per-core worker event loops

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod mtu;
pub mod numa;
pub mod udp;
pub mod worker;

// AF_XDP and io_uring are Linux-specific
#[cfg(target_os = "linux")]
pub mod af_xdp;

#[cfg(target_os = "linux")]
pub mod io_uring_impl;

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Use kernel bypass (AF_XDP) if available
    pub use_xdp: bool,
    /// Number of worker threads (0 = auto-detect)
    pub workers: usize,
    /// Receive buffer size
    pub recv_buffer_size: usize,
    /// Send buffer size
    pub send_buffer_size: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            use_xdp: true,
            workers: 0,
            recv_buffer_size: 256 * 1024,
            send_buffer_size: 256 * 1024,
        }
    }
}
