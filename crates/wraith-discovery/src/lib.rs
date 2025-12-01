//! # WRAITH Discovery
//!
//! Peer discovery layer for the WRAITH protocol.
//!
//! This crate provides:
//! - Privacy-enhanced DHT (encrypted announcements)
//! - DERP-style relay network for NAT traversal
//! - NAT type detection and hole punching
//! - Endpoint discovery
//!
//! ## Kademlia DHT
//!
//! The DHT module implements a privacy-enhanced Kademlia DHT with:
//! - 256-bit node identifiers (BLAKE3 hash of public keys)
//! - XOR distance metric
//! - K-bucket routing (k=20)
//! - Encrypted messages (XChaCha20-Poly1305)
//! - Iterative lookup with alpha parallelism
//!
//! ## Example
//!
//! ```rust,no_run
//! use wraith_discovery::dht::{DhtNode, NodeId};
//! use std::time::Duration;
//!
//! let id = NodeId::random();
//! let addr = "127.0.0.1:8000".parse().unwrap();
//! let mut node = DhtNode::new(id, addr);
//!
//! // Store a value
//! node.store([42u8; 32], vec![1, 2, 3], Duration::from_secs(3600));
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod dht;
pub mod manager;
pub mod nat;
pub mod relay;

// Re-export commonly used types
pub use manager::{
    ConnectionType, DiscoveryConfig, DiscoveryError, DiscoveryManager, DiscoveryState,
    PeerConnection, RelayInfo,
};
pub use nat::{
    Candidate, CandidateType, HolePuncher, IceGatherer, NatDetector, NatError, NatType, PunchError,
    StunClient, StunError,
};

/// Peer endpoint information
#[derive(Debug, Clone)]
pub struct PeerEndpoint {
    /// Peer public key
    pub public_key: [u8; 32],
    /// Direct endpoints (IP:port pairs)
    pub endpoints: Vec<std::net::SocketAddr>,
    /// Relay endpoints if direct connection fails
    pub relay_endpoints: Vec<String>,
}
