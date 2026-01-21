//! NAT Traversal Module
//!
//! This module provides NAT traversal capabilities for the WRAITH protocol,
//! enabling direct peer-to-peer connections through NAT devices.
//!
//! # Components
//!
//! - **NAT Type Detection**: Identifies the type of NAT device (Full Cone, Restricted Cone, etc.)
//! - **STUN Client**: Implements STUN protocol (RFC 5389) for server reflexive address discovery
//! - **ICE Candidate Gathering**: Collects host, server reflexive, and relay candidates
//! - **UDP Hole Punching**: Establishes direct connections through NAT using simultaneous open
//!
//! # NAT Types
//!
//! The module classifies NAT devices into categories based on their behavior:
//!
//! - **Open**: No NAT, direct public IP connectivity
//! - **Full Cone**: Any external host can send packets to the mapped port
//! - **Restricted Cone**: Only hosts that received packets can send back
//! - **Port Restricted Cone**: Only specific host:port pairs can send back
//! - **Symmetric**: Different external mapping for each destination
//!
//! # Example
//!
//! ```rust,no_run
//! use wraith_discovery::nat::{NatDetector, HolePuncher};
//! use std::net::SocketAddr;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Detect NAT type
//! let detector = NatDetector::new();
//! let nat_type = detector.detect().await?;
//! println!("NAT type: {:?}", nat_type);
//!
//! // Perform hole punching
//! let local_addr = "0.0.0.0:0".parse::<SocketAddr>()?;
//! let puncher = HolePuncher::new(local_addr).await?;
//! let peer_addr = "203.0.113.10:12345".parse()?;
//! let connection = puncher.punch(peer_addr, None).await?;
//! println!("Connected to peer at: {}", connection);
//! # Ok(())
//! # }
//! ```

pub mod dns;
pub mod hole_punch;
pub mod ice;
pub mod signaling;
pub mod stun;
pub mod types;

// Re-exports
pub use dns::{DnsError, StunDnsResolver, StunServerSpec, default_stun_servers, fallback_stun_ips};
pub use hole_punch::{HolePuncher, PunchError};
pub use ice::{Candidate, CandidateType, IceCandidate, IceGatherer};
pub use signaling::{
    CandidatePair, ConnectivityChecker, NatSignaling, PairState, SerializableCandidate,
    SignalingError, SignalingMessage,
};
pub use stun::{
    StunAttribute, StunAuthentication, StunClient, StunError, StunMessage, StunMessageClass,
    StunMessageType, StunRateLimiter,
};
pub use types::{NatDetector, NatError, NatType};
