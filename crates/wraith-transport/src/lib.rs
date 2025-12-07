//! # WRAITH Transport
//!
//! Network transport layer for the WRAITH protocol.
//!
//! This crate provides:
//! - Transport trait abstraction for multiple backends
//! - Async UDP transport using Tokio
//! - QUIC transport (placeholder for future implementation)
//! - Transport factory for configuration-based creation
//! - AF_XDP socket management for zero-copy packet I/O (Linux-only)
//! - io_uring integration for async file operations (Linux-only)
//! - UDP socket fallback for non-Linux systems
//! - Per-core worker event loops

#![warn(missing_docs)]
#![warn(clippy::all)]

// Transport trait and implementations
pub mod factory;
pub mod quic;
pub mod transport;
pub mod udp_async;

// Legacy sync UDP transport
pub mod udp;

// Kernel bypass and async I/O
pub mod buffer_pool;
pub mod io_uring;
pub mod mtu;
pub mod numa;
pub mod worker;

// Re-export BufferPool at crate root for convenience
pub use buffer_pool::BufferPool;

// AF_XDP is Linux-specific
#[cfg(target_os = "linux")]
pub mod af_xdp;

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
