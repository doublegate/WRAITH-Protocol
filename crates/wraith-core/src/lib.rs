//! # WRAITH Core
//!
//! Core protocol implementation for the WRAITH (Wire-speed Resilient Authenticated
//! Invisible Transfer Handler) protocol.
//!
//! This crate provides:
//! - Frame encoding and decoding (zero-copy parsing)
//! - Session state machine
//! - Stream multiplexing
//! - BBR congestion control
//! - Error types and handling
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         Session                                  │
//! │   (authenticated, encrypted connection between two peers)       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                         Streams                                  │
//! │   (multiplexed logical channels for file transfers)             │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                         Frames                                   │
//! │   (encrypted protocol data units with padding)                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod congestion;
pub mod error;
pub mod frame;
pub mod session;
pub mod stream;

pub use congestion::BbrState;
pub use error::Error;
pub use frame::{Frame, FrameBuilder, FrameFlags, FrameType};
pub use session::{Session, SessionConfig, SessionState};
pub use stream::Stream;

/// Protocol version (major.minor encoded as u32)
pub const PROTOCOL_VERSION: u32 = 0x00000001;

/// Fixed frame header size in bytes
pub const FRAME_HEADER_SIZE: usize = 28;

/// AEAD authentication tag size
pub const AUTH_TAG_SIZE: usize = 16;

/// Connection ID size
pub const CONNECTION_ID_SIZE: usize = 8;
