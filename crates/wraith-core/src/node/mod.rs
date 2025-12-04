//! Node orchestration layer for WRAITH Protocol
//!
//! This module provides the high-level Node API that coordinates all protocol
//! components:
//! - Cryptographic handshakes (Noise_XX via wraith-crypto)
//! - Transport selection (AF_XDP/UDP via wraith-transport)
//! - Obfuscation (padding/timing via wraith-obfuscation)
//! - Peer discovery (DHT/NAT via wraith-discovery)
//! - File transfer (chunking/hashing via wraith-files)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     Node API                             │
//! │  (High-level orchestration, user-facing interface)      │
//! ├─────────────────────────────────────────────────────────┤
//! │  Sessions  │  Transfers  │  Discovery  │  Transport     │
//! ├─────────────────────────────────────────────────────────┤
//! │  Crypto    │  Obfuscation │  Files     │  Networking    │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use wraith_core::node::Node;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create node with random identity
//!     let node = Node::new_random().await?;
//!
//!     // Connect to peer and send file
//!     let peer_id = [0u8; 32]; // Peer's public key
//!     let transfer_id = node.send_file("document.pdf", &peer_id).await?;
//!
//!     // Wait for completion
//!     node.wait_for_transfer(transfer_id).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod connection;
pub mod discovery;
pub mod error;
pub mod nat;
#[allow(clippy::module_inception)]
pub mod node;
pub mod obfuscation;
pub mod session;
pub mod transfer;

pub use config::NodeConfig;
pub use connection::{HealthMetrics, HealthStatus};
pub use discovery::{NatType, NodeCapabilities, PeerAnnouncement, PeerInfo};
pub use error::NodeError;
pub use nat::{CandidateType, IceCandidate};
pub use node::Node;
pub use obfuscation::{ObfuscationStats, Protocol};
pub use session::PeerConnection;
pub use transfer::FileMetadata;
