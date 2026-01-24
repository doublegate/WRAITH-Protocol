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
//! # Module Structure
//!
//! - [`node`] - Main Node struct and lifecycle management
//! - [`identity`] - Identity management (Ed25519 + X25519 keys)
//! - [`session_manager`] - Session lifecycle management
//! - [`transfer_manager`] - File transfer coordination
//! - [`session`] - PeerConnection and handshake functions
//! - [`config`] - Configuration types
//! - [`error`] - Error types
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

// Re-export BufferPool from wraith-transport for backward compatibility
// The buffer pool is now defined in wraith-transport where it's primarily used
pub use wraith_transport::BufferPool;

pub mod circuit_breaker;
pub mod config;
pub mod connection;
pub mod discovery;
pub mod error;
pub mod file_transfer;
pub mod health;
pub mod ice;
pub mod identity;
pub mod ip_reputation;
pub mod multi_peer;
pub mod nat;
#[allow(clippy::module_inception)]
pub mod node;
pub mod obfuscation;
pub mod packet_handler;
pub mod padding_strategy;
pub mod progress;
pub mod rate_limiter;
pub mod resume;
pub mod routing;
pub mod security_monitor;
pub mod session;
pub mod session_manager;
pub mod transfer;
pub mod transfer_manager;

// BufferPool is re-exported from wraith_transport at the top of this module
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitMetrics, CircuitState, RetryConfig,
};
pub use config::{
    CoverTrafficConfig, CoverTrafficDistribution, DiscoveryConfig, LogLevel, LoggingConfig,
    MimicryMode, NodeConfig, ObfuscationConfig, PaddingMode, TimingMode, TransferConfig,
    TransportConfig,
};
pub use connection::{HealthMetrics, HealthStatus};
pub use discovery::{NatType, NodeCapabilities, PeerAnnouncement, PeerInfo};
pub use error::{NodeError, Result};
pub use file_transfer::{FileMetadata, FileTransferContext};
pub use health::{HealthAction, HealthConfig, HealthMonitor};
pub use ice::{
    CandidatePair, CandidateType as IceCandidateType, CheckList, CheckState, IceAgent,
    IceCandidate as FullIceCandidate, IceConfig, IceCredentials, IceError, IceResult, IceRole,
    IceState, IceStats, IceStatsSnapshot, TransportProtocol, TurnServer,
};
pub use identity::{Identity, TransferId};
pub use ip_reputation::{
    IpReputationConfig, IpReputationMetrics, IpReputationSystem, ReputationStatus,
};
pub use multi_peer::{ChunkAssignmentStrategy, MultiPeerCoordinator, PeerPerformance};
pub use nat::{CandidateType, IceAgentDiagnostics, IceCandidate};
pub use node::Node;
pub use obfuscation::{ObfuscationStats, Protocol};
pub use padding_strategy::{
    ConstantRatePadding, NonePadding, PaddingStrategy, PowerOfTwoPadding, SizeClassesPadding,
    StatisticalPadding, create_padding_strategy,
};
pub use progress::{TransferProgress, TransferStatus};
pub use rate_limiter::{RateLimitConfig, RateLimitMetrics, RateLimiter};
pub use resume::{ResumeManager, ResumeState};
pub use routing::{RoutingStats, RoutingTable, extract_connection_id};
pub use security_monitor::{
    SecurityEvent, SecurityEventCallback, SecurityEventType, SecurityMetrics, SecurityMonitor,
    SecurityMonitorConfig,
};
pub use session::PeerConnection;
pub use session_manager::SessionManager;
pub use transfer_manager::TransferManager;
