//! # WRAITH Discovery
//!
//! Peer discovery layer for the WRAITH protocol.
//!
//! This crate provides:
//! - Privacy-enhanced DHT (encrypted announcements)
//! - DERP-style relay network for NAT traversal
//! - NAT type detection and hole punching
//! - Endpoint discovery

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod dht;
pub mod nat;
pub mod relay;

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
