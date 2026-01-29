//! # WRAITH Core
//!
//! Core protocol implementation for the WRAITH (Wire-speed Resilient Authenticated
//! Invisible Transfer Handler) protocol.
//!
//! This crate provides:
//! - **Node API**: High-level protocol orchestration layer
//! - **Frame encoding and decoding**: Zero-copy parsing with padding
//! - **Session state machine**: Noise_XX handshake and session lifecycle
//! - **Stream multiplexing**: Logical channels for concurrent file transfers
//! - **BBR congestion control**: Bandwidth-aware congestion control
//! - **Transfer session management**: Multi-peer file transfer coordination
//! - **Error types and handling**: Comprehensive error management
//!
//! ## Quick Start
//!
//! The [`Node`] API is the primary entry point for using WRAITH:
//!
//! ```no_run
//! use wraith_core::Node;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a new node with random identity
//!     let node = Node::new_random().await?;
//!
//!     // Start the node
//!     node.start().await?;
//!
//!     // Send a file to a peer
//!     let peer_id = [0u8; 32]; // Peer's Ed25519 public key
//!     let transfer_id = node.send_file("document.pdf", &peer_id).await?;
//!
//!     // Wait for transfer completion
//!     node.wait_for_transfer(transfer_id).await?;
//!
//!     // Stop the node
//!     node.stop().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      Node (Orchestration)                       │
//! │  - Session management, peer discovery, file transfers           │
//! ├─────────────────────────────────────────────────────────────────┤
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
//!
//! ## Module Structure
//!
//! - [`node`]: High-level Node API for protocol orchestration
//! - [`session`]: Session state machine and lifecycle management
//! - [`stream`]: Stream multiplexing for concurrent transfers
//! - [`frame`]: Frame encoding/decoding and protocol data units
//! - [`congestion`]: BBR congestion control implementation
//! - [`transfer`]: File transfer session management
//! - [`migration`]: Connection migration and multi-path support
//! - [`path`]: MTU discovery and path management
//! - [`error`]: Error types and result handling

#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod congestion;
pub mod error;
pub mod frame;
pub mod migration;
pub mod node;
pub mod path;
pub mod ring_buffer;
pub mod session;
pub mod stream;
pub mod transfer;

pub use congestion::BbrState;
pub use error::Error;
pub use frame::{Frame, FrameBuilder, FrameFlags, FrameType, build_into_from_parts};
pub use migration::{PathState, PathValidator, ValidatedPath};
pub use node::{Node, NodeConfig, NodeError};
pub use path::{DEFAULT_MTU, MAX_MTU, MIN_MTU, PathMtuDiscovery};
pub use ring_buffer::{MpscRingBuffer, SpscRingBuffer};
pub use session::{
    ConnectionId, HandshakePhase, Session, SessionConfig, SessionState, SessionStats,
};
pub use stream::{Stream, StreamState};

/// Protocol version (major.minor encoded as u32)
pub const PROTOCOL_VERSION: u32 = 0x0000_0001;

/// Fixed frame header size in bytes
pub const FRAME_HEADER_SIZE: usize = 28;

/// AEAD authentication tag size
pub const AUTH_TAG_SIZE: usize = 16;

/// Connection ID size
pub const CONNECTION_ID_SIZE: usize = 8;
