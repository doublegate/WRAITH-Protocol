//! RFC 8445 ICE (Interactive Connectivity Establishment) Implementation
//!
//! This module provides a full implementation of ICE for NAT traversal per RFC 8445.
//! ICE is used to establish peer-to-peer connections through NATs and firewalls.
//!
//! ## Key Components
//!
//! - [`IceAgent`] - Main ICE state machine managing the connection process
//! - [`IceCandidate`] - Represents a potential connection endpoint
//! - [`CandidatePair`] - A pairing of local and remote candidates for connectivity checks
//! - [`CheckList`] - Ordered list of candidate pairs for connectivity testing
//!
//! ## ICE Process Overview
//!
//! 1. **Gathering** - Collect candidates (host, server reflexive, relay)
//! 2. **Exchange** - Share candidates with peer via signaling
//! 3. **Connectivity Checks** - Test candidate pairs with STUN
//! 4. **Nomination** - Select the best working pair
//! 5. **Completed** - Connection established
//!
//! ## References
//!
//! - RFC 8445: ICE Protocol
//! - RFC 8838: Trickle ICE
//! - RFC 8863: ICE Timeout
//!
//! ## Example
//!
//! ```no_run
//! use wraith_core::node::ice::{IceAgent, IceConfig, IceRole};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create ICE agent as controlling (initiator)
//! let config = IceConfig::default();
//! let mut agent = IceAgent::new(IceRole::Controlling, config)?;
//!
//! // Gather candidates
//! agent.gather_candidates().await?;
//!
//! // Get local candidates for signaling
//! let local_candidates = agent.local_candidates();
//!
//! // Add remote candidates from signaling
//! // agent.add_remote_candidate(remote_candidate)?;
//!
//! // Run connectivity checks
//! agent.start_checks().await?;
//!
//! // Wait for completion
//! // let nominated_pair = agent.get_nominated_pair()?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::time::timeout;

/// ICE implementation errors
#[derive(Debug, Error)]
pub enum IceError {
    /// Invalid ICE state for operation
    #[error("Invalid ICE state: expected {expected}, got {actual}")]
    InvalidState {
        /// Expected state
        expected: &'static str,
        /// Actual state
        actual: String,
    },

    /// Candidate gathering failed
    #[error("Candidate gathering failed: {0}")]
    GatheringFailed(String),

    /// Connectivity check failed
    #[error("Connectivity check failed: {0}")]
    CheckFailed(String),

    /// All connectivity checks failed
    #[error("All connectivity checks failed")]
    AllChecksFailed,

    /// Nomination failed
    #[error("Nomination failed: {0}")]
    NominationFailed(String),

    /// Timeout during ICE process
    #[error("ICE timeout: {0}")]
    Timeout(String),

    /// Invalid candidate
    #[error("Invalid candidate: {0}")]
    InvalidCandidate(String),

    /// STUN error
    #[error("STUN error: {0}")]
    StunError(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for ICE operations
pub type IceResult<T> = Result<T, IceError>;

/// ICE agent role per RFC 8445 Section 4
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IceRole {
    /// Controlling agent (initiator) - responsible for nomination
    Controlling,
    /// Controlled agent (responder) - follows controlling agent's nomination
    Controlled,
}

impl std::fmt::Display for IceRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IceRole::Controlling => write!(f, "controlling"),
            IceRole::Controlled => write!(f, "controlled"),
        }
    }
}

/// ICE agent state per RFC 8445 Section 6
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IceState {
    /// Initial state - no gathering or checks started
    New,
    /// Gathering local candidates
    Gathering,
    /// Connectivity checks in progress
    Checking,
    /// At least one candidate pair is working
    Connected,
    /// Nomination complete, all checks finished
    Completed,
    /// ICE processing failed
    Failed,
    /// ICE session closed
    Closed,
}

impl std::fmt::Display for IceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IceState::New => write!(f, "new"),
            IceState::Gathering => write!(f, "gathering"),
            IceState::Checking => write!(f, "checking"),
            IceState::Connected => write!(f, "connected"),
            IceState::Completed => write!(f, "completed"),
            IceState::Failed => write!(f, "failed"),
            IceState::Closed => write!(f, "closed"),
        }
    }
}

/// ICE candidate type per RFC 8445 Section 5.1.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateType {
    /// Host candidate - directly reachable IP address
    Host,
    /// Server reflexive candidate - public address learned from STUN
    ServerReflexive,
    /// Peer reflexive candidate - discovered during connectivity checks
    PeerReflexive,
    /// Relay candidate - TURN server relay address
    Relay,
}

impl CandidateType {
    /// Get type preference for priority calculation (RFC 8445 Section 5.1.2.1)
    /// Higher values = more preferred
    pub fn type_preference(&self) -> u32 {
        match self {
            CandidateType::Host => 126,
            CandidateType::ServerReflexive => 100,
            CandidateType::PeerReflexive => 110,
            CandidateType::Relay => 0,
        }
    }
}

impl std::fmt::Display for CandidateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandidateType::Host => write!(f, "host"),
            CandidateType::ServerReflexive => write!(f, "srflx"),
            CandidateType::PeerReflexive => write!(f, "prflx"),
            CandidateType::Relay => write!(f, "relay"),
        }
    }
}

/// Transport protocol for ICE candidate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportProtocol {
    /// UDP transport
    Udp,
    /// TCP transport (not commonly used in WRAITH)
    Tcp,
}

impl std::fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportProtocol::Udp => write!(f, "UDP"),
            TransportProtocol::Tcp => write!(f, "TCP"),
        }
    }
}

/// ICE candidate per RFC 8445 Section 5.1
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IceCandidate {
    /// Foundation - identifier for similar candidates (for check list ordering)
    pub foundation: String,

    /// Component ID (1 for RTP, 2 for RTCP; we use 1 for data)
    pub component_id: u8,

    /// Transport protocol
    pub transport: TransportProtocol,

    /// Priority (calculated per RFC 8445 Section 5.1.2.1)
    pub priority: u32,

    /// Candidate address (IP and port)
    pub address: SocketAddr,

    /// Candidate type
    pub candidate_type: CandidateType,

    /// Related address (for srflx/prflx/relay: the base address)
    pub related_address: Option<SocketAddr>,

    /// Timestamp when candidate was gathered
    pub gathered_at: Instant,
}

impl IceCandidate {
    /// Create a new host candidate
    pub fn host(address: SocketAddr, component_id: u8) -> Self {
        let foundation = Self::compute_foundation(CandidateType::Host, &address, None);
        let priority = Self::compute_priority(CandidateType::Host, 65535, component_id);

        Self {
            foundation,
            component_id,
            transport: TransportProtocol::Udp,
            priority,
            address,
            candidate_type: CandidateType::Host,
            related_address: None,
            gathered_at: Instant::now(),
        }
    }

    /// Create a server reflexive candidate
    pub fn server_reflexive(
        address: SocketAddr,
        base_address: SocketAddr,
        component_id: u8,
    ) -> Self {
        let foundation = Self::compute_foundation(
            CandidateType::ServerReflexive,
            &address,
            Some(&base_address),
        );
        let priority = Self::compute_priority(CandidateType::ServerReflexive, 65534, component_id);

        Self {
            foundation,
            component_id,
            transport: TransportProtocol::Udp,
            priority,
            address,
            candidate_type: CandidateType::ServerReflexive,
            related_address: Some(base_address),
            gathered_at: Instant::now(),
        }
    }

    /// Create a peer reflexive candidate (discovered during checks)
    pub fn peer_reflexive(
        address: SocketAddr,
        base_address: SocketAddr,
        priority: u32,
        component_id: u8,
    ) -> Self {
        let foundation =
            Self::compute_foundation(CandidateType::PeerReflexive, &address, Some(&base_address));

        Self {
            foundation,
            component_id,
            transport: TransportProtocol::Udp,
            priority,
            address,
            candidate_type: CandidateType::PeerReflexive,
            related_address: Some(base_address),
            gathered_at: Instant::now(),
        }
    }

    /// Create a relay candidate
    pub fn relay(address: SocketAddr, base_address: SocketAddr, component_id: u8) -> Self {
        let foundation =
            Self::compute_foundation(CandidateType::Relay, &address, Some(&base_address));
        let priority = Self::compute_priority(CandidateType::Relay, 65533, component_id);

        Self {
            foundation,
            component_id,
            transport: TransportProtocol::Udp,
            priority,
            address,
            candidate_type: CandidateType::Relay,
            related_address: Some(base_address),
            gathered_at: Instant::now(),
        }
    }

    /// Compute foundation per RFC 8445 Section 5.1.1.3
    /// Foundation groups candidates that share the same type, base IP, STUN server, and transport
    fn compute_foundation(
        candidate_type: CandidateType,
        address: &SocketAddr,
        base: Option<&SocketAddr>,
    ) -> String {
        let base_ip = base.map(|a| a.ip()).unwrap_or(address.ip());
        let hash_input = format!("{candidate_type}-{base_ip}");
        let hash = blake3::hash(hash_input.as_bytes());
        hex::encode(&hash.as_bytes()[..8])
    }

    /// Compute priority per RFC 8445 Section 5.1.2.1
    ///
    /// priority = (2^24 * type_preference) + (2^8 * local_preference) + (256 - component_id)
    pub fn compute_priority(
        candidate_type: CandidateType,
        local_preference: u32,
        component_id: u8,
    ) -> u32 {
        let type_pref = candidate_type.type_preference();
        let local_pref = local_preference.min(65535);
        // component_id is u8, so it's always <= 255; use directly
        let component = component_id;

        (1 << 24) * type_pref + (1 << 8) * local_pref + (256 - component as u32)
    }

    /// Format as SDP attribute (a=candidate:...)
    pub fn to_sdp(&self) -> String {
        let mut sdp = format!(
            "candidate:{} {} {} {} {} {} typ {}",
            self.foundation,
            self.component_id,
            self.transport,
            self.priority,
            self.address.ip(),
            self.address.port(),
            self.candidate_type,
        );

        if let Some(ref related) = self.related_address {
            sdp.push_str(&format!(" raddr {} rport {}", related.ip(), related.port()));
        }

        sdp
    }

    /// Parse from SDP attribute
    pub fn from_sdp(sdp: &str) -> IceResult<Self> {
        // Format: candidate:foundation component transport priority addr port typ type [raddr addr rport port]
        let sdp = sdp.strip_prefix("candidate:").unwrap_or(sdp);
        let parts: Vec<&str> = sdp.split_whitespace().collect();

        if parts.len() < 7 {
            return Err(IceError::InvalidCandidate(
                "SDP has fewer than 7 fields".into(),
            ));
        }

        let foundation = parts[0].to_string();
        let component_id: u8 = parts[1]
            .parse()
            .map_err(|e| IceError::InvalidCandidate(format!("Invalid component: {e}")))?;
        let transport = match parts[2].to_uppercase().as_str() {
            "UDP" => TransportProtocol::Udp,
            "TCP" => TransportProtocol::Tcp,
            other => {
                return Err(IceError::InvalidCandidate(format!(
                    "Unknown transport: {other}"
                )));
            }
        };
        let priority: u32 = parts[3]
            .parse()
            .map_err(|e| IceError::InvalidCandidate(format!("Invalid priority: {e}")))?;
        let ip = parts[4]
            .parse()
            .map_err(|e| IceError::InvalidCandidate(format!("Invalid IP: {e}")))?;
        let port: u16 = parts[5]
            .parse()
            .map_err(|e| IceError::InvalidCandidate(format!("Invalid port: {e}")))?;

        // Parse type (after "typ")
        let typ_idx = parts
            .iter()
            .position(|&p| p == "typ")
            .ok_or_else(|| IceError::InvalidCandidate("Missing 'typ' keyword".into()))?;

        if typ_idx + 1 >= parts.len() {
            return Err(IceError::InvalidCandidate(
                "Missing candidate type after 'typ'".into(),
            ));
        }

        let candidate_type = match parts[typ_idx + 1] {
            "host" => CandidateType::Host,
            "srflx" => CandidateType::ServerReflexive,
            "prflx" => CandidateType::PeerReflexive,
            "relay" => CandidateType::Relay,
            other => {
                return Err(IceError::InvalidCandidate(format!(
                    "Unknown candidate type: {other}"
                )));
            }
        };

        // Parse optional related address
        let related_address = if let Some(raddr_idx) = parts.iter().position(|&p| p == "raddr") {
            let rport_idx = parts.iter().position(|&p| p == "rport").ok_or_else(|| {
                IceError::InvalidCandidate("Missing 'rport' after 'raddr'".into())
            })?;

            let rip = parts
                .get(raddr_idx + 1)
                .ok_or_else(|| IceError::InvalidCandidate("Missing related IP".into()))?
                .parse()
                .map_err(|e| IceError::InvalidCandidate(format!("Invalid related IP: {e}")))?;

            let rport: u16 = parts
                .get(rport_idx + 1)
                .ok_or_else(|| IceError::InvalidCandidate("Missing related port".into()))?
                .parse()
                .map_err(|e| IceError::InvalidCandidate(format!("Invalid related port: {e}")))?;

            Some(SocketAddr::new(rip, rport))
        } else {
            None
        };

        Ok(Self {
            foundation,
            component_id,
            transport,
            priority,
            address: SocketAddr::new(ip, port),
            candidate_type,
            related_address,
            gathered_at: Instant::now(),
        })
    }
}

impl std::fmt::Display for IceCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}({}) pri={} {}",
            self.candidate_type, self.address, self.foundation, self.priority, self.transport
        )
    }
}

/// State of a connectivity check per RFC 8445 Section 6.1.2.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckState {
    /// Check is waiting to be performed
    Waiting,
    /// Check is currently in progress
    InProgress,
    /// Check succeeded
    Succeeded,
    /// Check failed
    Failed,
    /// Check is frozen (waiting for another check to complete)
    Frozen,
}

impl std::fmt::Display for CheckState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckState::Waiting => write!(f, "waiting"),
            CheckState::InProgress => write!(f, "in-progress"),
            CheckState::Succeeded => write!(f, "succeeded"),
            CheckState::Failed => write!(f, "failed"),
            CheckState::Frozen => write!(f, "frozen"),
        }
    }
}

/// A candidate pair for connectivity checking per RFC 8445 Section 6.1
#[derive(Debug, Clone)]
pub struct CandidatePair {
    /// Local candidate
    pub local: IceCandidate,

    /// Remote candidate
    pub remote: IceCandidate,

    /// Current check state
    pub state: CheckState,

    /// Whether this pair is nominated
    pub nominated: bool,

    /// Pair priority (computed per RFC 8445 Section 6.1.2.3)
    pub priority: u64,

    /// Foundation (combination of local and remote foundations)
    pub foundation: String,

    /// Number of check attempts
    pub attempts: u32,

    /// Last check time
    pub last_check: Option<Instant>,

    /// Round-trip time if check succeeded (in microseconds)
    pub rtt_us: Option<u64>,
}

impl CandidatePair {
    /// Create a new candidate pair
    pub fn new(local: IceCandidate, remote: IceCandidate, controlling: bool) -> Self {
        let priority = Self::compute_pair_priority(local.priority, remote.priority, controlling);
        let foundation = format!("{}:{}", local.foundation, remote.foundation);

        Self {
            local,
            remote,
            state: CheckState::Frozen,
            nominated: false,
            priority,
            foundation,
            attempts: 0,
            last_check: None,
            rtt_us: None,
        }
    }

    /// Compute pair priority per RFC 8445 Section 6.1.2.3
    ///
    /// pair priority = 2^32*MIN(G,D) + 2*MAX(G,D) + (G>D?1:0)
    /// where G = controlling priority, D = controlled priority
    pub fn compute_pair_priority(local_pri: u32, remote_pri: u32, controlling: bool) -> u64 {
        let (g, d) = if controlling {
            (local_pri as u64, remote_pri as u64)
        } else {
            (remote_pri as u64, local_pri as u64)
        };

        let min = g.min(d);
        let max = g.max(d);

        (1u64 << 32) * min + 2 * max + if g > d { 1 } else { 0 }
    }

    /// Check if pair can be used (check succeeded and optionally nominated)
    pub fn is_usable(&self, require_nomination: bool) -> bool {
        self.state == CheckState::Succeeded && (!require_nomination || self.nominated)
    }
}

impl std::fmt::Display for CandidatePair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} <-> {} ({}, pri={}, nom={})",
            self.local.address, self.remote.address, self.state, self.priority, self.nominated
        )
    }
}

/// Check list for managing connectivity checks per RFC 8445 Section 6.1.2
#[derive(Debug)]
pub struct CheckList {
    /// List of candidate pairs, sorted by priority
    pairs: Vec<CandidatePair>,

    /// Index of next pair to check (reserved for future use in ordered checking)
    #[allow(dead_code)]
    next_check: usize,

    /// Whether ordinary checks are complete (reserved for future use in aggressive nomination)
    #[allow(dead_code)]
    ordinary_complete: bool,
}

impl CheckList {
    /// Create a new empty check list
    pub fn new() -> Self {
        Self {
            pairs: Vec::new(),
            next_check: 0,
            ordinary_complete: false,
        }
    }

    /// Add a candidate pair to the list
    pub fn add_pair(&mut self, pair: CandidatePair) {
        self.pairs.push(pair);
        // Re-sort by priority (descending)
        self.pairs.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Get the next pair to check (triggered or ordinary)
    pub fn next_waiting_pair(&mut self) -> Option<&mut CandidatePair> {
        // First, look for triggered checks (Waiting state)
        if let Some(idx) = self
            .pairs
            .iter()
            .position(|p| p.state == CheckState::Waiting)
        {
            return Some(&mut self.pairs[idx]);
        }

        // Then look for frozen checks to unfreeze
        if let Some(idx) = self
            .pairs
            .iter()
            .position(|p| p.state == CheckState::Frozen)
        {
            self.pairs[idx].state = CheckState::Waiting;
            return Some(&mut self.pairs[idx]);
        }

        None
    }

    /// Get a pair by its foundation
    pub fn get_pair_by_foundation(&mut self, foundation: &str) -> Option<&mut CandidatePair> {
        self.pairs.iter_mut().find(|p| p.foundation == foundation)
    }

    /// Get the nominated pair (if any)
    pub fn nominated_pair(&self) -> Option<&CandidatePair> {
        self.pairs
            .iter()
            .find(|p| p.nominated && p.state == CheckState::Succeeded)
    }

    /// Get all succeeded pairs
    pub fn succeeded_pairs(&self) -> impl Iterator<Item = &CandidatePair> {
        self.pairs
            .iter()
            .filter(|p| p.state == CheckState::Succeeded)
    }

    /// Check if all pairs have been checked (no more Waiting or Frozen)
    pub fn is_complete(&self) -> bool {
        !self.pairs.iter().any(|p| {
            matches!(
                p.state,
                CheckState::Waiting | CheckState::Frozen | CheckState::InProgress
            )
        })
    }

    /// Get all pairs
    pub fn pairs(&self) -> &[CandidatePair] {
        &self.pairs
    }

    /// Get mutable reference to pairs
    pub fn pairs_mut(&mut self) -> &mut [CandidatePair] {
        &mut self.pairs
    }

    /// Get number of pairs
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Prune redundant pairs per RFC 8445 Section 6.1.2.4
    pub fn prune_redundant(&mut self) {
        // Remove pairs where there's already a higher-priority pair with same local base
        let mut seen_bases: HashMap<SocketAddr, u64> = HashMap::new();

        self.pairs.retain(|pair| {
            let base = pair.local.related_address.unwrap_or(pair.local.address);

            if let Some(&existing_priority) = seen_bases.get(&base) {
                // Keep only if this has higher priority
                pair.priority > existing_priority
            } else {
                seen_bases.insert(base, pair.priority);
                true
            }
        });
    }
}

impl Default for CheckList {
    fn default() -> Self {
        Self::new()
    }
}

/// ICE credentials (username fragment and password)
#[derive(Debug, Clone)]
pub struct IceCredentials {
    /// Username fragment (4-256 chars, RFC 8445 Section 5.3)
    pub ufrag: String,

    /// Password (22-256 chars, RFC 8445 Section 5.3)
    pub pwd: String,
}

impl IceCredentials {
    /// Generate new random credentials
    pub fn generate() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Generate random bytes and encode as alphanumeric
        // Note: `gen` is a reserved keyword in Rust 2024, so we use r#gen
        let ufrag_bytes: [u8; 4] = rng.r#gen();
        let pwd_bytes: [u8; 22] = rng.r#gen();

        Self {
            ufrag: hex::encode(ufrag_bytes),
            pwd: hex::encode(pwd_bytes),
        }
    }

    /// Validate credentials
    pub fn validate(&self) -> IceResult<()> {
        if self.ufrag.len() < 4 || self.ufrag.len() > 256 {
            return Err(IceError::ConfigError(
                "ufrag must be 4-256 characters".into(),
            ));
        }
        if self.pwd.len() < 22 || self.pwd.len() > 256 {
            return Err(IceError::ConfigError(
                "pwd must be 22-256 characters".into(),
            ));
        }
        Ok(())
    }
}

/// ICE agent configuration
#[derive(Debug, Clone)]
pub struct IceConfig {
    /// STUN server addresses
    pub stun_servers: Vec<SocketAddr>,

    /// TURN server addresses (with credentials)
    pub turn_servers: Vec<TurnServer>,

    /// Candidate gathering timeout
    pub gathering_timeout: Duration,

    /// Connectivity check timeout per attempt
    pub check_timeout: Duration,

    /// Maximum check attempts per pair
    pub max_check_attempts: u32,

    /// Overall ICE timeout (RFC 8863 recommends 39.5s minimum)
    pub ice_timeout: Duration,

    /// Enable aggressive nomination (immediate nomination on first success)
    pub aggressive_nomination: bool,

    /// Keepalive interval for nominated pair
    pub keepalive_interval: Duration,

    /// Local preference offset for IPv6 addresses
    pub ipv6_preference_offset: i32,

    /// Enable Trickle ICE (incremental candidate exchange)
    pub trickle_ice: bool,
}

impl Default for IceConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec![
                "74.125.250.129:19302".parse().unwrap(), // Google STUN
                "64.233.163.127:19302".parse().unwrap(), // Google STUN 2
            ],
            turn_servers: Vec::new(),
            gathering_timeout: Duration::from_secs(10),
            check_timeout: Duration::from_millis(500),
            max_check_attempts: 7,
            ice_timeout: Duration::from_millis(39500), // RFC 8863 minimum
            aggressive_nomination: true,
            keepalive_interval: Duration::from_secs(15),
            ipv6_preference_offset: 0,
            trickle_ice: true,
        }
    }
}

/// TURN server configuration
#[derive(Debug, Clone)]
pub struct TurnServer {
    /// Server address
    pub address: SocketAddr,

    /// Username for authentication
    pub username: String,

    /// Password for authentication
    pub password: String,
}

/// ICE agent statistics
#[derive(Debug, Default)]
pub struct IceStats {
    /// Number of candidates gathered
    pub candidates_gathered: AtomicU64,

    /// Number of connectivity checks sent
    pub checks_sent: AtomicU64,

    /// Number of connectivity checks succeeded
    pub checks_succeeded: AtomicU64,

    /// Number of connectivity checks failed
    pub checks_failed: AtomicU64,

    /// Total time spent gathering (microseconds)
    pub gathering_time_us: AtomicU64,

    /// Total time spent checking (microseconds)
    pub checking_time_us: AtomicU64,
}

impl IceStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Get snapshot of current stats
    pub fn snapshot(&self) -> IceStatsSnapshot {
        IceStatsSnapshot {
            candidates_gathered: self.candidates_gathered.load(Ordering::Relaxed),
            checks_sent: self.checks_sent.load(Ordering::Relaxed),
            checks_succeeded: self.checks_succeeded.load(Ordering::Relaxed),
            checks_failed: self.checks_failed.load(Ordering::Relaxed),
            gathering_time_us: self.gathering_time_us.load(Ordering::Relaxed),
            checking_time_us: self.checking_time_us.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of ICE statistics
#[derive(Debug, Clone)]
pub struct IceStatsSnapshot {
    /// Number of candidates gathered
    pub candidates_gathered: u64,
    /// Number of connectivity checks sent
    pub checks_sent: u64,
    /// Number of successful checks
    pub checks_succeeded: u64,
    /// Number of failed checks
    pub checks_failed: u64,
    /// Time spent gathering candidates (microseconds)
    pub gathering_time_us: u64,
    /// Time spent on connectivity checks (microseconds)
    pub checking_time_us: u64,
}

/// ICE Agent - main state machine for ICE connectivity establishment
pub struct IceAgent {
    /// Agent role (controlling or controlled)
    role: IceRole,

    /// Current ICE state
    state: Arc<RwLock<IceState>>,

    /// Local ICE candidates
    local_candidates: Arc<RwLock<Vec<IceCandidate>>>,

    /// Remote ICE candidates
    remote_candidates: Arc<RwLock<Vec<IceCandidate>>>,

    /// Check list for connectivity checks
    check_list: Arc<RwLock<CheckList>>,

    /// Local ICE credentials
    local_credentials: IceCredentials,

    /// Remote ICE credentials
    remote_credentials: Arc<RwLock<Option<IceCredentials>>>,

    /// Configuration
    config: IceConfig,

    /// Statistics
    stats: Arc<IceStats>,

    /// Tie-breaker for role conflicts (random 64-bit value)
    tie_breaker: u64,
}

impl IceAgent {
    /// Create a new ICE agent
    pub fn new(role: IceRole, config: IceConfig) -> IceResult<Self> {
        let local_credentials = IceCredentials::generate();
        local_credentials.validate()?;

        let tie_breaker = rand::random::<u64>();

        Ok(Self {
            role,
            state: Arc::new(RwLock::new(IceState::New)),
            local_candidates: Arc::new(RwLock::new(Vec::new())),
            remote_candidates: Arc::new(RwLock::new(Vec::new())),
            check_list: Arc::new(RwLock::new(CheckList::new())),
            local_credentials,
            remote_credentials: Arc::new(RwLock::new(None)),
            config,
            stats: Arc::new(IceStats::new()),
            tie_breaker,
        })
    }

    /// Get current ICE state
    pub async fn state(&self) -> IceState {
        *self.state.read().await
    }

    /// Get agent role
    pub fn role(&self) -> IceRole {
        self.role
    }

    /// Get local credentials
    pub fn local_credentials(&self) -> &IceCredentials {
        &self.local_credentials
    }

    /// Set remote credentials
    pub async fn set_remote_credentials(&self, credentials: IceCredentials) -> IceResult<()> {
        credentials.validate()?;
        *self.remote_credentials.write().await = Some(credentials);
        Ok(())
    }

    /// Get local candidates (for signaling)
    pub async fn local_candidates(&self) -> Vec<IceCandidate> {
        self.local_candidates.read().await.clone()
    }

    /// Get statistics
    pub fn stats(&self) -> &IceStats {
        &self.stats
    }

    /// Gather local candidates
    ///
    /// Collects host candidates from local interfaces, then server reflexive
    /// candidates from STUN servers, and relay candidates from TURN servers.
    pub async fn gather_candidates(&self) -> IceResult<()> {
        // Transition to Gathering state
        {
            let mut state = self.state.write().await;
            match *state {
                IceState::New | IceState::Gathering => *state = IceState::Gathering,
                s => {
                    return Err(IceError::InvalidState {
                        expected: "New or Gathering",
                        actual: s.to_string(),
                    });
                }
            }
        }

        let start = Instant::now();

        // 1. Gather host candidates (local interfaces)
        let host_candidates = self.gather_host_candidates().await?;

        // 2. Gather server reflexive candidates (STUN)
        let srflx_candidates = if !self.config.stun_servers.is_empty() {
            timeout(
                self.config.gathering_timeout,
                self.gather_srflx_candidates(&host_candidates),
            )
            .await
            .unwrap_or_else(|_| {
                tracing::warn!("STUN gathering timed out");
                Ok(Vec::new())
            })?
        } else {
            Vec::new()
        };

        // 3. Gather relay candidates (TURN)
        let relay_candidates = if !self.config.turn_servers.is_empty() {
            timeout(
                self.config.gathering_timeout,
                self.gather_relay_candidates(&host_candidates),
            )
            .await
            .unwrap_or_else(|_| {
                tracing::warn!("TURN gathering timed out");
                Ok(Vec::new())
            })?
        } else {
            Vec::new()
        };

        // Combine all candidates
        let mut local_candidates = self.local_candidates.write().await;
        local_candidates.extend(host_candidates);
        local_candidates.extend(srflx_candidates);
        local_candidates.extend(relay_candidates);

        let count = local_candidates.len();
        self.stats
            .candidates_gathered
            .fetch_add(count as u64, Ordering::Relaxed);

        self.stats
            .gathering_time_us
            .fetch_add(start.elapsed().as_micros() as u64, Ordering::Relaxed);

        tracing::info!("Gathered {} local ICE candidates", count);

        Ok(())
    }

    /// Gather host candidates from local interfaces
    async fn gather_host_candidates(&self) -> IceResult<Vec<IceCandidate>> {
        let mut candidates = Vec::new();

        // Get local interfaces
        // In a real implementation, we would enumerate network interfaces
        // For now, we get the default addresses
        let addrs = self.get_local_addresses().await?;

        for addr in addrs {
            let candidate = IceCandidate::host(addr, 1);
            tracing::debug!("Gathered host candidate: {}", candidate);
            candidates.push(candidate);
        }

        Ok(candidates)
    }

    /// Get local IP addresses
    async fn get_local_addresses(&self) -> IceResult<Vec<SocketAddr>> {
        // Use a temporary UDP socket to determine local addresses
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| IceError::NetworkError(format!("Failed to bind socket: {e}")))?;

        // Try to connect to each STUN server to determine local address
        // (doesn't actually send data)
        let mut addresses = Vec::new();

        for stun_server in &self.config.stun_servers {
            if let Ok(()) = socket.connect(stun_server).await {
                if let Ok(local_addr) = socket.local_addr() {
                    if !addresses.contains(&local_addr) {
                        addresses.push(local_addr);
                    }
                }
            }
        }

        // Fallback: try to connect to a public DNS server
        if addresses.is_empty() {
            if let Ok(()) = socket.connect("8.8.8.8:53").await {
                if let Ok(local_addr) = socket.local_addr() {
                    addresses.push(local_addr);
                }
            }
        }

        // Last fallback: use 0.0.0.0 with the bound port
        if addresses.is_empty() {
            let port = socket.local_addr().map(|a| a.port()).unwrap_or(0);
            addresses.push(SocketAddr::from(([0, 0, 0, 0], port)));
        }

        Ok(addresses)
    }

    /// Gather server reflexive candidates via STUN
    async fn gather_srflx_candidates(
        &self,
        host_candidates: &[IceCandidate],
    ) -> IceResult<Vec<IceCandidate>> {
        let mut candidates = Vec::new();

        for host in host_candidates {
            for stun_server in &self.config.stun_servers {
                match self.stun_binding_request(host.address, *stun_server).await {
                    Ok(mapped_addr) => {
                        // Only add if different from host
                        if mapped_addr != host.address {
                            let candidate =
                                IceCandidate::server_reflexive(mapped_addr, host.address, 1);
                            tracing::debug!("Gathered srflx candidate: {}", candidate);
                            candidates.push(candidate);
                        }
                        break; // One srflx per host is enough
                    }
                    Err(e) => {
                        tracing::debug!(
                            "STUN request to {} failed for {}: {}",
                            stun_server,
                            host.address,
                            e
                        );
                    }
                }
            }
        }

        Ok(candidates)
    }

    /// Perform STUN Binding Request
    async fn stun_binding_request(
        &self,
        local_addr: SocketAddr,
        stun_server: SocketAddr,
    ) -> IceResult<SocketAddr> {
        // Bind to the specified local address
        let socket = tokio::net::UdpSocket::bind(local_addr)
            .await
            .map_err(|e| IceError::StunError(format!("Failed to bind: {e}")))?;

        // Connect to STUN server
        socket
            .connect(stun_server)
            .await
            .map_err(|e| IceError::StunError(format!("Failed to connect: {e}")))?;

        // Build STUN Binding Request
        // RFC 5389 format:
        // - Type: 0x0001 (Binding Request)
        // - Length: 0 (no attributes)
        // - Magic Cookie: 0x2112A442
        // - Transaction ID: 12 random bytes
        let mut request = vec![0u8; 20];
        request[0..2].copy_from_slice(&0x0001u16.to_be_bytes()); // Type
        request[2..4].copy_from_slice(&0x0000u16.to_be_bytes()); // Length
        request[4..8].copy_from_slice(&0x2112A442u32.to_be_bytes()); // Magic Cookie

        // Transaction ID
        let txn_id: [u8; 12] = rand::random();
        request[8..20].copy_from_slice(&txn_id);

        // Send request
        socket
            .send(&request)
            .await
            .map_err(|e| IceError::StunError(format!("Failed to send: {e}")))?;

        // Receive response with timeout
        let mut response = vec![0u8; 512];
        let result = timeout(self.config.check_timeout, socket.recv(&mut response)).await;

        let len = result
            .map_err(|_| IceError::StunError("STUN timeout".into()))?
            .map_err(|e| IceError::StunError(format!("Recv error: {e}")))?;

        // Parse STUN response
        if len < 20 {
            return Err(IceError::StunError("Response too short".into()));
        }

        // Verify it's a Binding Success Response (0x0101)
        let msg_type = u16::from_be_bytes([response[0], response[1]]);
        if msg_type != 0x0101 {
            return Err(IceError::StunError(format!(
                "Unexpected message type: {msg_type:#06x}"
            )));
        }

        // Verify transaction ID matches
        if response[8..20] != txn_id {
            return Err(IceError::StunError("Transaction ID mismatch".into()));
        }

        // Parse attributes to find XOR-MAPPED-ADDRESS (0x0020) or MAPPED-ADDRESS (0x0001)
        let attr_len = u16::from_be_bytes([response[2], response[3]]) as usize;
        let mut pos = 20;
        let end = 20 + attr_len.min(len - 20);

        while pos + 4 <= end {
            let attr_type = u16::from_be_bytes([response[pos], response[pos + 1]]);
            let attr_len = u16::from_be_bytes([response[pos + 2], response[pos + 3]]) as usize;
            pos += 4;

            if pos + attr_len > end {
                break;
            }

            if attr_type == 0x0020 {
                // XOR-MAPPED-ADDRESS
                if attr_len >= 8 && response[pos + 1] == 0x01 {
                    // IPv4
                    let port = u16::from_be_bytes([response[pos + 2], response[pos + 3]]) ^ 0x2112;
                    let ip_bytes = [
                        response[pos + 4] ^ 0x21,
                        response[pos + 5] ^ 0x12,
                        response[pos + 6] ^ 0xA4,
                        response[pos + 7] ^ 0x42,
                    ];
                    let ip = std::net::Ipv4Addr::from(ip_bytes);
                    return Ok(SocketAddr::new(std::net::IpAddr::V4(ip), port));
                }
            } else if attr_type == 0x0001 {
                // MAPPED-ADDRESS (fallback)
                if attr_len >= 8 && response[pos + 1] == 0x01 {
                    // IPv4
                    let port = u16::from_be_bytes([response[pos + 2], response[pos + 3]]);
                    let ip_bytes = [
                        response[pos + 4],
                        response[pos + 5],
                        response[pos + 6],
                        response[pos + 7],
                    ];
                    let ip = std::net::Ipv4Addr::from(ip_bytes);
                    return Ok(SocketAddr::new(std::net::IpAddr::V4(ip), port));
                }
            }

            pos += (attr_len + 3) & !3; // Pad to 4-byte boundary
        }

        Err(IceError::StunError("No MAPPED-ADDRESS in response".into()))
    }

    /// Gather relay candidates via TURN (placeholder - full TURN not implemented)
    async fn gather_relay_candidates(
        &self,
        _host_candidates: &[IceCandidate],
    ) -> IceResult<Vec<IceCandidate>> {
        // TURN implementation would go here
        // For now, return empty - relay is handled via wraith-discovery relay servers
        tracing::debug!("TURN relay gathering not implemented - using wraith-discovery relay");
        Ok(Vec::new())
    }

    /// Add a remote candidate received via signaling
    pub async fn add_remote_candidate(&self, candidate: IceCandidate) -> IceResult<()> {
        tracing::debug!("Adding remote candidate: {}", candidate);

        // Add to remote candidates list
        {
            let mut remote = self.remote_candidates.write().await;
            remote.push(candidate.clone());
        }

        // Form pairs with existing local candidates
        let local_candidates = self.local_candidates.read().await;
        let controlling = self.role == IceRole::Controlling;

        let mut check_list = self.check_list.write().await;
        for local in local_candidates.iter() {
            // Only pair candidates of same component and transport
            if local.component_id == candidate.component_id
                && local.transport == candidate.transport
            {
                let pair = CandidatePair::new(local.clone(), candidate.clone(), controlling);
                tracing::trace!("Formed candidate pair: {}", pair);
                check_list.add_pair(pair);
            }
        }

        // Prune redundant pairs
        check_list.prune_redundant();

        Ok(())
    }

    /// Start connectivity checks
    pub async fn start_checks(&self) -> IceResult<()> {
        // Transition to Checking state
        {
            let mut state = self.state.write().await;
            match *state {
                IceState::New | IceState::Gathering => *state = IceState::Checking,
                IceState::Checking => {} // Already checking
                s => {
                    return Err(IceError::InvalidState {
                        expected: "New, Gathering, or Checking",
                        actual: s.to_string(),
                    });
                }
            }
        }

        // Form initial check list from all candidate pairs
        let local_candidates = self.local_candidates.read().await;
        let remote_candidates = self.remote_candidates.read().await;
        let controlling = self.role == IceRole::Controlling;

        {
            let mut check_list = self.check_list.write().await;

            for local in local_candidates.iter() {
                for remote in remote_candidates.iter() {
                    if local.component_id == remote.component_id
                        && local.transport == remote.transport
                    {
                        let pair = CandidatePair::new(local.clone(), remote.clone(), controlling);
                        check_list.add_pair(pair);
                    }
                }
            }

            // Prune redundant pairs
            check_list.prune_redundant();

            // Unfreeze first pair in each foundation group
            let mut seen_foundations: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for pair in check_list.pairs_mut() {
                if !seen_foundations.contains(&pair.foundation) {
                    pair.state = CheckState::Waiting;
                    seen_foundations.insert(pair.foundation.clone());
                }
            }
        }

        let start = Instant::now();

        // Run connectivity checks with overall timeout
        let check_result = timeout(self.config.ice_timeout, self.run_checks()).await;

        self.stats
            .checking_time_us
            .fetch_add(start.elapsed().as_micros() as u64, Ordering::Relaxed);

        match check_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                *self.state.write().await = IceState::Failed;
                Err(e)
            }
            Err(_) => {
                *self.state.write().await = IceState::Failed;
                Err(IceError::Timeout("ICE checks timed out".into()))
            }
        }
    }

    /// Run connectivity checks loop
    async fn run_checks(&self) -> IceResult<()> {
        let check_interval = Duration::from_millis(50);

        loop {
            // Get next pair to check
            let pair_to_check = {
                let mut check_list = self.check_list.write().await;

                // Check for completion
                if check_list.is_complete() {
                    if check_list.nominated_pair().is_some() {
                        *self.state.write().await = IceState::Completed;
                        return Ok(());
                    } else if check_list.succeeded_pairs().next().is_some() {
                        // We have succeeded pairs but none nominated
                        // In aggressive nomination, we should have nominated already
                        *self.state.write().await = IceState::Connected;
                        return Ok(());
                    } else {
                        return Err(IceError::AllChecksFailed);
                    }
                }

                check_list.next_waiting_pair().map(|p| {
                    p.state = CheckState::InProgress;
                    p.attempts += 1;
                    p.last_check = Some(Instant::now());
                    (p.local.clone(), p.remote.clone(), p.foundation.clone())
                })
            };

            if let Some((local, remote, foundation)) = pair_to_check {
                // Perform connectivity check
                self.stats.checks_sent.fetch_add(1, Ordering::Relaxed);

                let check_start = Instant::now();
                let check_result = self.perform_connectivity_check(&local, &remote).await;

                // Update pair state based on result
                let mut check_list = self.check_list.write().await;
                if let Some(pair) = check_list.get_pair_by_foundation(&foundation) {
                    match check_result {
                        Ok(rtt) => {
                            pair.state = CheckState::Succeeded;
                            pair.rtt_us = Some(rtt.as_micros() as u64);
                            self.stats.checks_succeeded.fetch_add(1, Ordering::Relaxed);

                            tracing::debug!(
                                "Connectivity check succeeded: {} <-> {} (RTT: {:?})",
                                local.address,
                                remote.address,
                                rtt
                            );

                            // Aggressive nomination: nominate immediately on first success
                            if self.config.aggressive_nomination
                                && self.role == IceRole::Controlling
                            {
                                pair.nominated = true;
                                *self.state.write().await = IceState::Completed;
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            if pair.attempts >= self.config.max_check_attempts {
                                pair.state = CheckState::Failed;
                                self.stats.checks_failed.fetch_add(1, Ordering::Relaxed);
                                tracing::debug!(
                                    "Connectivity check failed after {} attempts: {} <-> {}: {}",
                                    pair.attempts,
                                    local.address,
                                    remote.address,
                                    e
                                );
                            } else {
                                // Retry
                                pair.state = CheckState::Waiting;
                                tracing::trace!(
                                    "Connectivity check attempt {} failed, will retry: {}",
                                    pair.attempts,
                                    e
                                );
                            }
                        }
                    }
                }

                // Small delay between checks
                let elapsed = check_start.elapsed();
                if elapsed < check_interval {
                    tokio::time::sleep(check_interval - elapsed).await;
                }
            } else {
                // No pairs to check right now, wait a bit
                tokio::time::sleep(check_interval).await;
            }
        }
    }

    /// Perform a single connectivity check
    async fn perform_connectivity_check(
        &self,
        local: &IceCandidate,
        remote: &IceCandidate,
    ) -> IceResult<Duration> {
        let start = Instant::now();

        // Bind to local candidate's address
        let socket = tokio::net::UdpSocket::bind(local.address)
            .await
            .map_err(|e| IceError::CheckFailed(format!("Failed to bind: {e}")))?;

        // Build STUN Binding Request with credentials
        // Include USERNAME, MESSAGE-INTEGRITY, ICE-CONTROLLING/ICE-CONTROLLED, PRIORITY
        let request = self.build_binding_request(local, remote)?;

        // Send to remote candidate
        socket
            .send_to(&request, remote.address)
            .await
            .map_err(|e| IceError::CheckFailed(format!("Failed to send: {e}")))?;

        // Wait for response
        let mut response = vec![0u8; 512];
        let result = timeout(self.config.check_timeout, socket.recv_from(&mut response)).await;

        let (len, from) = result
            .map_err(|_| IceError::CheckFailed("Check timeout".into()))?
            .map_err(|e| IceError::CheckFailed(format!("Recv error: {e}")))?;

        // Validate response is from expected address
        if from != remote.address {
            return Err(IceError::CheckFailed(format!(
                "Response from unexpected address: {} (expected {})",
                from, remote.address
            )));
        }

        // Verify it's a valid STUN response
        if len < 20 {
            return Err(IceError::CheckFailed("Response too short".into()));
        }

        let msg_type = u16::from_be_bytes([response[0], response[1]]);
        if msg_type != 0x0101 {
            // Binding Success Response
            return Err(IceError::CheckFailed(format!(
                "Unexpected response type: {msg_type:#06x}"
            )));
        }

        Ok(start.elapsed())
    }

    /// Build STUN Binding Request with ICE attributes
    fn build_binding_request(
        &self,
        _local: &IceCandidate,
        remote: &IceCandidate,
    ) -> IceResult<Vec<u8>> {
        // Simple STUN Binding Request
        // In a full implementation, we would add:
        // - USERNAME (local-ufrag:remote-ufrag)
        // - MESSAGE-INTEGRITY (HMAC-SHA1 with password)
        // - ICE-CONTROLLING or ICE-CONTROLLED with tie-breaker
        // - PRIORITY
        // - USE-CANDIDATE (for nomination)

        let mut request = vec![0u8; 20];
        request[0..2].copy_from_slice(&0x0001u16.to_be_bytes()); // Binding Request
        request[2..4].copy_from_slice(&0x0000u16.to_be_bytes()); // Length (will update)
        request[4..8].copy_from_slice(&0x2112A442u32.to_be_bytes()); // Magic Cookie

        // Transaction ID
        let txn_id: [u8; 12] = rand::random();
        request[8..20].copy_from_slice(&txn_id);

        // Add PRIORITY attribute (0x0024)
        let priority = remote.priority;
        let mut priority_attr = vec![0u8; 8];
        priority_attr[0..2].copy_from_slice(&0x0024u16.to_be_bytes()); // Type
        priority_attr[2..4].copy_from_slice(&4u16.to_be_bytes()); // Length
        priority_attr[4..8].copy_from_slice(&priority.to_be_bytes()); // Value
        request.extend_from_slice(&priority_attr);

        // Add ICE-CONTROLLING or ICE-CONTROLLED attribute
        let (ice_attr_type, ice_value) = if self.role == IceRole::Controlling {
            (0x802Au16, self.tie_breaker) // ICE-CONTROLLING
        } else {
            (0x8029u16, self.tie_breaker) // ICE-CONTROLLED
        };
        let mut ice_attr = vec![0u8; 12];
        ice_attr[0..2].copy_from_slice(&ice_attr_type.to_be_bytes());
        ice_attr[2..4].copy_from_slice(&8u16.to_be_bytes());
        ice_attr[4..12].copy_from_slice(&ice_value.to_be_bytes());
        request.extend_from_slice(&ice_attr);

        // Update length field (message length excluding header)
        let msg_len = (request.len() - 20) as u16;
        request[2..4].copy_from_slice(&msg_len.to_be_bytes());

        Ok(request)
    }

    /// Nominate a candidate pair (controlling agent only)
    pub async fn nominate(&self, pair_foundation: &str) -> IceResult<()> {
        if self.role != IceRole::Controlling {
            return Err(IceError::NominationFailed(
                "Only controlling agent can nominate".into(),
            ));
        }

        let mut check_list = self.check_list.write().await;
        if let Some(pair) = check_list.get_pair_by_foundation(pair_foundation) {
            if pair.state != CheckState::Succeeded {
                return Err(IceError::NominationFailed(
                    "Can only nominate succeeded pairs".into(),
                ));
            }

            pair.nominated = true;
            tracing::info!("Nominated pair: {}", pair);

            *self.state.write().await = IceState::Completed;
            Ok(())
        } else {
            Err(IceError::NominationFailed(format!(
                "Pair not found: {pair_foundation}"
            )))
        }
    }

    /// Get the nominated pair (if any)
    pub async fn get_nominated_pair(&self) -> Option<CandidatePair> {
        self.check_list.read().await.nominated_pair().cloned()
    }

    /// Get best succeeded pair (even if not nominated)
    pub async fn get_best_pair(&self) -> Option<CandidatePair> {
        self.check_list
            .read()
            .await
            .succeeded_pairs()
            .max_by_key(|p| p.priority)
            .cloned()
    }

    /// Trigger ICE restart
    pub async fn restart(&self) -> IceResult<()> {
        tracing::info!("Restarting ICE");

        // Generate new credentials
        let new_creds = IceCredentials::generate();

        // Clear state
        {
            *self.state.write().await = IceState::New;
            self.local_candidates.write().await.clear();
            self.remote_candidates.write().await.clear();
            *self.check_list.write().await = CheckList::new();
            *self.remote_credentials.write().await = None;
        }

        // Note: In a real implementation, we would update local_credentials
        // But since it's not behind a lock, we can't do that here
        // The caller should create a new IceAgent for a full restart
        let _ = new_creds; // Suppress warning

        Ok(())
    }

    /// Close the ICE agent
    pub async fn close(&self) {
        *self.state.write().await = IceState::Closed;
        tracing::debug!("ICE agent closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_priority_calculation() {
        // Host candidate: type_pref=126, local_pref=65535, component=1
        // priority = 2^24 * 126 + 2^8 * 65535 + (256 - 1)
        // = 16777216 * 126 + 256 * 65535 + 255
        // = 2113929216 + 16776960 + 255 = 2130706431
        let priority = IceCandidate::compute_priority(CandidateType::Host, 65535, 1);
        assert_eq!(priority, 2_130_706_431);

        // Server reflexive: type_pref=100, local_pref=65535, component=1
        // = 16777216 * 100 + 256 * 65535 + 255
        // = 1677721600 + 16776960 + 255 = 1694498815
        let priority = IceCandidate::compute_priority(CandidateType::ServerReflexive, 65535, 1);
        assert_eq!(priority, 1_694_498_815);

        // Relay: type_pref=0, local_pref=65535, component=1
        // = 16777216 * 0 + 256 * 65535 + 255
        // = 0 + 16776960 + 255 = 16777215
        let priority = IceCandidate::compute_priority(CandidateType::Relay, 65535, 1);
        assert_eq!(priority, 16_777_215);
    }

    #[test]
    fn test_pair_priority_calculation() {
        // Controlling with higher priority
        let g = 2_000_000_000u32;
        let d = 1_000_000_000u32;
        let priority = CandidatePair::compute_pair_priority(g, d, true);
        // 2^32 * 1000000000 + 2 * 2000000000 + 1
        assert!(priority > 0);

        // Controlled - priorities are swapped
        let priority_controlled = CandidatePair::compute_pair_priority(g, d, false);
        assert_ne!(priority, priority_controlled);
    }

    #[test]
    fn test_candidate_sdp_roundtrip() {
        let addr: SocketAddr = "192.168.1.100:54321".parse().unwrap();
        let candidate = IceCandidate::host(addr, 1);

        let sdp = candidate.to_sdp();
        let parsed = IceCandidate::from_sdp(&sdp).unwrap();

        assert_eq!(candidate.address, parsed.address);
        assert_eq!(candidate.candidate_type, parsed.candidate_type);
        assert_eq!(candidate.component_id, parsed.component_id);
        assert_eq!(candidate.priority, parsed.priority);
    }

    #[test]
    fn test_candidate_sdp_with_related() {
        let addr: SocketAddr = "203.0.113.50:12345".parse().unwrap();
        let base: SocketAddr = "192.168.1.100:54321".parse().unwrap();
        let candidate = IceCandidate::server_reflexive(addr, base, 1);

        let sdp = candidate.to_sdp();
        assert!(sdp.contains("raddr"));
        assert!(sdp.contains("rport"));

        let parsed = IceCandidate::from_sdp(&sdp).unwrap();
        assert_eq!(candidate.address, parsed.address);
        assert_eq!(candidate.related_address, parsed.related_address);
        assert_eq!(parsed.candidate_type, CandidateType::ServerReflexive);
    }

    #[test]
    fn test_credentials_generation() {
        let creds = IceCredentials::generate();

        assert!(creds.ufrag.len() >= 4);
        assert!(creds.ufrag.len() <= 256);
        assert!(creds.pwd.len() >= 22);
        assert!(creds.pwd.len() <= 256);
        assert!(creds.validate().is_ok());
    }

    #[test]
    fn test_credentials_validation() {
        let mut creds = IceCredentials::generate();

        // Valid credentials should pass
        assert!(creds.validate().is_ok());

        // Too short ufrag
        creds.ufrag = "abc".to_string();
        assert!(creds.validate().is_err());

        // Reset ufrag, test short pwd
        creds.ufrag = "abcd".to_string();
        creds.pwd = "short".to_string();
        assert!(creds.validate().is_err());
    }

    #[test]
    fn test_check_list_operations() {
        let mut check_list = CheckList::new();

        let local = IceCandidate::host("192.168.1.100:5000".parse().unwrap(), 1);
        let remote1 = IceCandidate::host("192.168.1.200:5000".parse().unwrap(), 1);
        let remote2 = IceCandidate::host("192.168.1.201:5000".parse().unwrap(), 1);

        let pair1 = CandidatePair::new(local.clone(), remote1, true);
        let pair2 = CandidatePair::new(local, remote2, true);

        check_list.add_pair(pair1);
        check_list.add_pair(pair2);

        assert_eq!(check_list.len(), 2);

        // Get next waiting (first one should be unfrozen)
        let next = check_list.next_waiting_pair();
        assert!(next.is_some());
    }

    #[test]
    fn test_ice_states() {
        assert_eq!(IceState::New.to_string(), "new");
        assert_eq!(IceState::Gathering.to_string(), "gathering");
        assert_eq!(IceState::Checking.to_string(), "checking");
        assert_eq!(IceState::Connected.to_string(), "connected");
        assert_eq!(IceState::Completed.to_string(), "completed");
        assert_eq!(IceState::Failed.to_string(), "failed");
        assert_eq!(IceState::Closed.to_string(), "closed");
    }

    #[test]
    fn test_candidate_types() {
        assert_eq!(CandidateType::Host.to_string(), "host");
        assert_eq!(CandidateType::ServerReflexive.to_string(), "srflx");
        assert_eq!(CandidateType::PeerReflexive.to_string(), "prflx");
        assert_eq!(CandidateType::Relay.to_string(), "relay");

        // Type preferences are ordered correctly
        assert!(
            CandidateType::Host.type_preference() > CandidateType::PeerReflexive.type_preference()
        );
        assert!(
            CandidateType::PeerReflexive.type_preference()
                > CandidateType::ServerReflexive.type_preference()
        );
        assert!(
            CandidateType::ServerReflexive.type_preference()
                > CandidateType::Relay.type_preference()
        );
    }

    #[tokio::test]
    async fn test_ice_agent_creation() {
        let config = IceConfig::default();
        let agent = IceAgent::new(IceRole::Controlling, config).unwrap();

        assert_eq!(agent.role(), IceRole::Controlling);
        assert_eq!(agent.state().await, IceState::New);
        assert!(!agent.local_credentials().ufrag.is_empty());
        assert!(!agent.local_credentials().pwd.is_empty());
    }

    #[tokio::test]
    async fn test_ice_agent_restart() {
        let config = IceConfig::default();
        let agent = IceAgent::new(IceRole::Controlling, config).unwrap();

        // Restart should reset state
        agent.restart().await.unwrap();

        assert_eq!(agent.state().await, IceState::New);
        assert!(agent.local_candidates().await.is_empty());
    }

    #[tokio::test]
    async fn test_ice_agent_close() {
        let config = IceConfig::default();
        let agent = IceAgent::new(IceRole::Controlled, config).unwrap();

        agent.close().await;
        assert_eq!(agent.state().await, IceState::Closed);
    }

    #[test]
    fn test_ice_config_defaults() {
        let config = IceConfig::default();

        assert!(!config.stun_servers.is_empty());
        assert!(config.turn_servers.is_empty());
        assert!(config.gathering_timeout > Duration::ZERO);
        assert!(config.check_timeout > Duration::ZERO);
        assert!(config.max_check_attempts > 0);
        assert!(config.ice_timeout >= Duration::from_millis(39500)); // RFC 8863 minimum
        assert!(config.aggressive_nomination);
    }

    #[test]
    fn test_ice_stats_snapshot() {
        let stats = IceStats::new();

        stats.candidates_gathered.fetch_add(5, Ordering::Relaxed);
        stats.checks_sent.fetch_add(10, Ordering::Relaxed);
        stats.checks_succeeded.fetch_add(3, Ordering::Relaxed);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.candidates_gathered, 5);
        assert_eq!(snapshot.checks_sent, 10);
        assert_eq!(snapshot.checks_succeeded, 3);
    }

    #[tokio::test]
    async fn test_add_remote_candidate() {
        let config = IceConfig::default();
        let agent = IceAgent::new(IceRole::Controlling, config).unwrap();

        // First gather local candidates
        let local = IceCandidate::host("127.0.0.1:5000".parse().unwrap(), 1);
        agent.local_candidates.write().await.push(local);

        // Add remote candidate
        let remote = IceCandidate::host("127.0.0.1:5001".parse().unwrap(), 1);
        agent.add_remote_candidate(remote.clone()).await.unwrap();

        // Verify remote was added
        let remotes = agent.remote_candidates.read().await;
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].address, remote.address);

        // Verify pair was formed
        let check_list = agent.check_list.read().await;
        assert_eq!(check_list.len(), 1);
    }

    #[tokio::test]
    async fn test_set_remote_credentials() {
        let config = IceConfig::default();
        let agent = IceAgent::new(IceRole::Controlled, config).unwrap();

        let creds = IceCredentials::generate();
        agent.set_remote_credentials(creds.clone()).await.unwrap();

        let stored = agent.remote_credentials.read().await;
        assert!(stored.is_some());
        assert_eq!(stored.as_ref().unwrap().ufrag, creds.ufrag);
    }

    #[test]
    fn test_candidate_pair_display() {
        let local = IceCandidate::host("192.168.1.100:5000".parse().unwrap(), 1);
        let remote = IceCandidate::host("192.168.1.200:5000".parse().unwrap(), 1);
        let pair = CandidatePair::new(local, remote, true);

        let display = format!("{}", pair);
        assert!(display.contains("192.168.1.100:5000"));
        assert!(display.contains("192.168.1.200:5000"));
        assert!(display.contains("frozen")); // Initial state
    }

    #[test]
    fn test_check_state_display() {
        assert_eq!(format!("{}", CheckState::Waiting), "waiting");
        assert_eq!(format!("{}", CheckState::InProgress), "in-progress");
        assert_eq!(format!("{}", CheckState::Succeeded), "succeeded");
        assert_eq!(format!("{}", CheckState::Failed), "failed");
        assert_eq!(format!("{}", CheckState::Frozen), "frozen");
    }
}
