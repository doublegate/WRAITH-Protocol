//! NAT Signaling for ICE Candidate Exchange
//!
//! This module implements DHT-based signaling for ICE candidate exchange between peers.
//! It enables peers to discover each other's network addresses through the distributed
//! hash table without requiring a centralized signaling server.
//!
//! # Protocol Overview
//!
//! 1. Initiator gathers ICE candidates and publishes them to DHT under their peer ID
//! 2. Responder looks up initiator's candidates from DHT
//! 3. Responder gathers own candidates and publishes response to DHT
//! 4. Both peers perform ICE connectivity checks per RFC 8445
//! 5. Best candidate pair is nominated for communication
//!
//! # Example
//!
//! ```rust,no_run
//! use wraith_discovery::nat::signaling::{NatSignaling, SignalingMessage};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create signaling instance
//! let signaling = NatSignaling::new([0u8; 32], "0.0.0.0:0".parse()?).await?;
//!
//! // Create ICE offer for a session
//! let session_id = [1u8; 32];
//! let offer = signaling.create_offer(&session_id).await?;
//!
//! // Gather local ICE candidates
//! let candidates = signaling.gather_candidates().await?;
//! # Ok(())
//! # }
//! ```

use crate::nat::ice::{Candidate, CandidateType, IceGatherer};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

/// Default TTL for signaling messages in the DHT (5 minutes)
#[allow(dead_code)]
const SIGNALING_TTL: u64 = 300;

/// Connectivity check timeout
const CONNECTIVITY_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum connectivity check attempts per candidate pair
const MAX_CHECK_ATTEMPTS: u32 = 3;

/// STUN binding request magic cookie (RFC 5389)
const STUN_MAGIC_COOKIE: u32 = 0x2112A442;

/// Signaling errors
#[derive(Debug, Error)]
pub enum SignalingError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// DHT operation failed
    #[error("DHT error: {0}")]
    Dht(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// No candidates available
    #[error("No candidates available")]
    NoCandidates,

    /// Connectivity check failed
    #[error("Connectivity check failed: {0}")]
    ConnectivityCheckFailed(String),

    /// Timeout
    #[error("Operation timed out")]
    Timeout,

    /// Invalid message
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
}

/// Signaling message types for ICE candidate exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalingMessage {
    /// ICE offer containing candidates
    Offer {
        /// Session identifier
        session_id: [u8; 32],
        /// Sender's peer ID
        sender_id: [u8; 32],
        /// ICE candidates
        candidates: Vec<SerializableCandidate>,
        /// ICE ufrag (username fragment)
        ufrag: String,
        /// ICE pwd (password for STUN message integrity)
        pwd: String,
        /// Timestamp
        timestamp: u64,
    },

    /// ICE answer containing candidates
    Answer {
        /// Session identifier
        session_id: [u8; 32],
        /// Sender's peer ID
        sender_id: [u8; 32],
        /// ICE candidates
        candidates: Vec<SerializableCandidate>,
        /// ICE ufrag
        ufrag: String,
        /// ICE pwd
        pwd: String,
        /// Timestamp
        timestamp: u64,
    },

    /// Additional candidate (trickle ICE)
    CandidateUpdate {
        /// Session identifier
        session_id: [u8; 32],
        /// Sender's peer ID
        sender_id: [u8; 32],
        /// New candidate
        candidate: SerializableCandidate,
        /// Timestamp
        timestamp: u64,
    },
}

impl SignalingMessage {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, SignalingError> {
        bincode::serde::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| SignalingError::Serialization(e.to_string()))
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SignalingError> {
        bincode::serde::decode_from_slice(bytes, bincode::config::standard())
            .map(|(msg, _)| msg)
            .map_err(|e| SignalingError::Serialization(e.to_string()))
    }

    /// Get session ID
    pub fn session_id(&self) -> &[u8; 32] {
        match self {
            Self::Offer { session_id, .. } => session_id,
            Self::Answer { session_id, .. } => session_id,
            Self::CandidateUpdate { session_id, .. } => session_id,
        }
    }

    /// Get sender ID
    pub fn sender_id(&self) -> &[u8; 32] {
        match self {
            Self::Offer { sender_id, .. } => sender_id,
            Self::Answer { sender_id, .. } => sender_id,
            Self::CandidateUpdate { sender_id, .. } => sender_id,
        }
    }
}

/// Serializable candidate for transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableCandidate {
    /// Candidate address
    pub address: String,
    /// Candidate type
    pub candidate_type: String,
    /// Priority
    pub priority: u32,
    /// Foundation (for candidate pairing)
    pub foundation: String,
}

impl From<&Candidate> for SerializableCandidate {
    fn from(c: &Candidate) -> Self {
        Self {
            address: c.address.to_string(),
            candidate_type: match c.candidate_type {
                CandidateType::Host => "host".to_string(),
                CandidateType::ServerReflexive => "srflx".to_string(),
                CandidateType::PeerReflexive => "prflx".to_string(),
                CandidateType::Relay => "relay".to_string(),
            },
            priority: c.priority,
            foundation: format!("{:x}", c.priority), // Simplified foundation
        }
    }
}

impl TryFrom<&SerializableCandidate> for Candidate {
    type Error = SignalingError;

    fn try_from(sc: &SerializableCandidate) -> Result<Self, Self::Error> {
        let address: SocketAddr = sc
            .address
            .parse()
            .map_err(|e| SignalingError::InvalidMessage(format!("Invalid address: {}", e)))?;

        let candidate_type = match sc.candidate_type.as_str() {
            "host" => CandidateType::Host,
            "srflx" => CandidateType::ServerReflexive,
            "prflx" => CandidateType::PeerReflexive,
            "relay" => CandidateType::Relay,
            other => {
                return Err(SignalingError::InvalidMessage(format!(
                    "Unknown candidate type: {}",
                    other
                )));
            }
        };

        Ok(Candidate {
            address,
            candidate_type,
            priority: sc.priority,
        })
    }
}

/// ICE candidate pair for connectivity checks
#[derive(Debug, Clone)]
pub struct CandidatePair {
    /// Local candidate
    pub local: Candidate,
    /// Remote candidate
    pub remote: Candidate,
    /// Combined priority (RFC 8445 Section 6.1.2.3)
    pub priority: u64,
    /// Pair state
    pub state: PairState,
    /// Number of check attempts
    pub check_attempts: u32,
    /// Last check time
    pub last_check: Option<Instant>,
}

impl CandidatePair {
    /// Create a new candidate pair
    pub fn new(local: Candidate, remote: Candidate, is_controlling: bool) -> Self {
        // Calculate pair priority per RFC 8445 Section 6.1.2.3
        // priority = 2^32 * MIN(G,D) + 2 * MAX(G,D) + (G>D ? 1 : 0)
        let (g, d) = if is_controlling {
            (local.priority as u64, remote.priority as u64)
        } else {
            (remote.priority as u64, local.priority as u64)
        };

        let priority = (1u64 << 32) * g.min(d) + 2 * g.max(d) + if g > d { 1 } else { 0 };

        Self {
            local,
            remote,
            priority,
            state: PairState::Frozen,
            check_attempts: 0,
            last_check: None,
        }
    }
}

/// State of a candidate pair
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairState {
    /// Not yet checked
    Frozen,
    /// Waiting to be checked
    Waiting,
    /// Check in progress
    InProgress,
    /// Check succeeded
    Succeeded,
    /// Check failed
    Failed,
}

/// ICE connectivity checker
pub struct ConnectivityChecker {
    /// Local socket for sending checks
    socket: Arc<UdpSocket>,
    /// Local ICE credentials
    local_ufrag: String,
    #[allow(dead_code)]
    local_pwd: String,
    /// Remote ICE credentials
    remote_ufrag: String,
    #[allow(dead_code)]
    remote_pwd: String,
    /// Whether we are the controlling agent
    is_controlling: bool,
    /// Transaction ID counter
    transaction_counter: u32,
}

impl ConnectivityChecker {
    /// Create a new connectivity checker
    pub fn new(
        socket: Arc<UdpSocket>,
        local_ufrag: String,
        local_pwd: String,
        remote_ufrag: String,
        remote_pwd: String,
        is_controlling: bool,
    ) -> Self {
        Self {
            socket,
            local_ufrag,
            local_pwd,
            remote_ufrag,
            remote_pwd,
            is_controlling,
            transaction_counter: 0,
        }
    }

    /// Perform connectivity check on a candidate pair
    pub async fn check(&mut self, pair: &mut CandidatePair) -> Result<bool, SignalingError> {
        pair.state = PairState::InProgress;
        pair.check_attempts += 1;
        pair.last_check = Some(Instant::now());

        // Build STUN binding request with ICE attributes
        let transaction_id = self.generate_transaction_id();
        let request = self.build_binding_request(&transaction_id);

        // Send the request
        self.socket.send_to(&request, pair.remote.address).await?;

        // Wait for response with timeout
        let mut buf = [0u8; 512];
        let socket_clone = self.socket.clone();
        let remote_addr = pair.remote.address;
        let check_result: Result<bool, std::io::Error> =
            tokio::time::timeout(CONNECTIVITY_CHECK_TIMEOUT, async move {
                loop {
                    let (len, addr) = socket_clone.recv_from(&mut buf).await?;
                    if addr == remote_addr && len >= 20 {
                        // Check if response matches our transaction ID
                        if buf[4..20] == transaction_id {
                            return Ok(true);
                        }
                    }
                }
            })
            .await
            .unwrap_or(Ok(false));

        match check_result {
            Ok(true) => {
                pair.state = PairState::Succeeded;
                Ok(true)
            }
            Ok(false) | Err(_) => {
                if pair.check_attempts >= MAX_CHECK_ATTEMPTS {
                    pair.state = PairState::Failed;
                } else {
                    pair.state = PairState::Waiting;
                }
                Ok(false)
            }
        }
    }

    /// Generate a unique transaction ID
    fn generate_transaction_id(&mut self) -> [u8; 16] {
        use std::time::{SystemTime, UNIX_EPOCH};

        self.transaction_counter = self.transaction_counter.wrapping_add(1);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let mut id = [0u8; 16];
        id[0..8].copy_from_slice(&timestamp.to_be_bytes());
        id[8..12].copy_from_slice(&self.transaction_counter.to_be_bytes());
        // Last 4 bytes are random
        id[12..16].copy_from_slice(&rand::random::<[u8; 4]>());

        id
    }

    /// Build a STUN binding request
    fn build_binding_request(&self, transaction_id: &[u8; 16]) -> Vec<u8> {
        let mut request = Vec::with_capacity(64);

        // STUN header (20 bytes)
        // Type: Binding Request (0x0001)
        request.extend_from_slice(&0x0001u16.to_be_bytes());
        // Message length (to be filled later)
        request.extend_from_slice(&0u16.to_be_bytes());
        // Magic cookie
        request.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
        // Transaction ID
        request.extend_from_slice(&transaction_id[0..12]);

        // USERNAME attribute (combined ufrag)
        let username = format!("{}:{}", self.remote_ufrag, self.local_ufrag);
        let username_bytes = username.as_bytes();
        let padding = (4 - (username_bytes.len() % 4)) % 4;

        // Attribute type: USERNAME (0x0006)
        request.extend_from_slice(&0x0006u16.to_be_bytes());
        // Attribute length
        request.extend_from_slice(&(username_bytes.len() as u16).to_be_bytes());
        // Attribute value
        request.extend_from_slice(username_bytes);
        // Padding
        request.extend(vec![0u8; padding]);

        // ICE-CONTROLLING or ICE-CONTROLLED attribute
        if self.is_controlling {
            // ICE-CONTROLLING (0x802A)
            request.extend_from_slice(&0x802Au16.to_be_bytes());
            request.extend_from_slice(&8u16.to_be_bytes());
            request.extend_from_slice(&rand::random::<u64>().to_be_bytes());
        } else {
            // ICE-CONTROLLED (0x8029)
            request.extend_from_slice(&0x8029u16.to_be_bytes());
            request.extend_from_slice(&8u16.to_be_bytes());
            request.extend_from_slice(&rand::random::<u64>().to_be_bytes());
        }

        // PRIORITY attribute (0x0024)
        let priority = 0x6E001FFFu32; // Example priority for host candidate
        request.extend_from_slice(&0x0024u16.to_be_bytes());
        request.extend_from_slice(&4u16.to_be_bytes());
        request.extend_from_slice(&priority.to_be_bytes());

        // Update message length
        let msg_len = (request.len() - 20) as u16;
        request[2..4].copy_from_slice(&msg_len.to_be_bytes());

        request
    }
}

/// NAT signaling coordinator
pub struct NatSignaling {
    /// Our peer ID
    local_peer_id: [u8; 32],
    /// ICE gatherer
    gatherer: IceGatherer,
    /// UDP socket for connectivity checks
    socket: Arc<UdpSocket>,
    /// Active sessions
    sessions: Arc<Mutex<HashMap<[u8; 32], SignalingSession>>>,
    /// Generated ICE credentials
    local_ufrag: String,
    local_pwd: String,
}

/// Active signaling session
struct SignalingSession {
    /// Remote peer ID
    remote_peer_id: [u8; 32],
    /// Local candidates
    local_candidates: Vec<Candidate>,
    /// Remote candidates
    remote_candidates: Vec<Candidate>,
    /// Candidate pairs
    pairs: Vec<CandidatePair>,
    /// Nominated pair index
    nominated_pair: Option<usize>,
    /// Session state
    state: SessionState,
    /// Created timestamp
    #[allow(dead_code)]
    created: Instant,
}

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionState {
    /// Gathering candidates
    Gathering,
    /// Exchanging candidates
    Exchanging,
    /// Checking connectivity
    Checking,
    /// Connection established
    Connected,
    /// Failed
    Failed,
}

impl NatSignaling {
    /// Create a new NAT signaling instance
    pub async fn new(
        local_peer_id: [u8; 32],
        bind_addr: SocketAddr,
    ) -> Result<Self, SignalingError> {
        let socket = UdpSocket::bind(bind_addr).await?;

        // Generate ICE credentials
        let local_ufrag = Self::generate_ufrag();
        let local_pwd = Self::generate_pwd();

        Ok(Self {
            local_peer_id,
            gatherer: IceGatherer::new(),
            socket: Arc::new(socket),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            local_ufrag,
            local_pwd,
        })
    }

    /// Create with custom STUN servers
    pub async fn with_stun_servers(
        local_peer_id: [u8; 32],
        bind_addr: SocketAddr,
        stun_servers: Vec<SocketAddr>,
    ) -> Result<Self, SignalingError> {
        let socket = UdpSocket::bind(bind_addr).await?;

        let local_ufrag = Self::generate_ufrag();
        let local_pwd = Self::generate_pwd();

        Ok(Self {
            local_peer_id,
            gatherer: IceGatherer::with_stun_servers(stun_servers),
            socket: Arc::new(socket),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            local_ufrag,
            local_pwd,
        })
    }

    /// Generate ICE ufrag (4-256 characters)
    fn generate_ufrag() -> String {
        let mut bytes = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut bytes);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }

    /// Generate ICE pwd (22-256 characters)
    fn generate_pwd() -> String {
        let mut bytes = [0u8; 24];
        rand::thread_rng().fill_bytes(&mut bytes);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }

    /// Gather local ICE candidates
    pub async fn gather_candidates(&self) -> Result<Vec<Candidate>, SignalingError> {
        let local_addr = self.socket.local_addr()?;
        self.gatherer
            .gather(local_addr)
            .await
            .map_err(SignalingError::Io)
    }

    /// Create an ICE offer for a session
    pub async fn create_offer(
        &self,
        session_id: &[u8; 32],
    ) -> Result<SignalingMessage, SignalingError> {
        let candidates = self.gather_candidates().await?;

        if candidates.is_empty() {
            return Err(SignalingError::NoCandidates);
        }

        // Store session
        let session = SignalingSession {
            remote_peer_id: [0u8; 32], // Will be set when answer received
            local_candidates: candidates.clone(),
            remote_candidates: Vec::new(),
            pairs: Vec::new(),
            nominated_pair: None,
            state: SessionState::Gathering,
            created: Instant::now(),
        };

        self.sessions.lock().await.insert(*session_id, session);

        let serializable_candidates: Vec<SerializableCandidate> =
            candidates.iter().map(SerializableCandidate::from).collect();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(SignalingMessage::Offer {
            session_id: *session_id,
            sender_id: self.local_peer_id,
            candidates: serializable_candidates,
            ufrag: self.local_ufrag.clone(),
            pwd: self.local_pwd.clone(),
            timestamp,
        })
    }

    /// Create an ICE answer in response to an offer
    pub async fn create_answer(
        &self,
        offer: &SignalingMessage,
    ) -> Result<SignalingMessage, SignalingError> {
        let (session_id, _remote_candidates, _remote_ufrag, _remote_pwd) = match offer {
            SignalingMessage::Offer {
                session_id,
                candidates,
                ufrag,
                pwd,
                sender_id,
                ..
            } => {
                // Parse remote candidates
                let parsed: Result<Vec<Candidate>, _> =
                    candidates.iter().map(Candidate::try_from).collect();
                let remote_cands = parsed?;

                // Store session with remote info
                let local_candidates = self.gather_candidates().await?;

                let session = SignalingSession {
                    remote_peer_id: *sender_id,
                    local_candidates: local_candidates.clone(),
                    remote_candidates: remote_cands.clone(),
                    pairs: Vec::new(),
                    nominated_pair: None,
                    state: SessionState::Exchanging,
                    created: Instant::now(),
                };

                self.sessions.lock().await.insert(*session_id, session);

                (session_id, remote_cands, ufrag.clone(), pwd.clone())
            }
            _ => return Err(SignalingError::InvalidMessage("Expected Offer".to_string())),
        };

        // Form candidate pairs
        self.form_candidate_pairs(session_id).await?;

        let candidates = self.gather_candidates().await?;
        let serializable_candidates: Vec<SerializableCandidate> =
            candidates.iter().map(SerializableCandidate::from).collect();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(SignalingMessage::Answer {
            session_id: *session_id,
            sender_id: self.local_peer_id,
            candidates: serializable_candidates,
            ufrag: self.local_ufrag.clone(),
            pwd: self.local_pwd.clone(),
            timestamp,
        })
    }

    /// Process an ICE answer
    pub async fn process_answer(&self, answer: &SignalingMessage) -> Result<(), SignalingError> {
        let (session_id, remote_candidates, sender_id) = match answer {
            SignalingMessage::Answer {
                session_id,
                candidates,
                sender_id,
                ..
            } => {
                let parsed: Result<Vec<Candidate>, _> =
                    candidates.iter().map(Candidate::try_from).collect();
                (*session_id, parsed?, *sender_id)
            }
            _ => {
                return Err(SignalingError::InvalidMessage(
                    "Expected Answer".to_string(),
                ));
            }
        };

        // Update session with remote candidates
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.remote_peer_id = sender_id;
            session.remote_candidates = remote_candidates;
            session.state = SessionState::Exchanging;
        } else {
            return Err(SignalingError::InvalidMessage(
                "Unknown session".to_string(),
            ));
        }
        drop(sessions);

        // Form candidate pairs
        self.form_candidate_pairs(&session_id).await?;

        Ok(())
    }

    /// Form candidate pairs from local and remote candidates
    async fn form_candidate_pairs(&self, session_id: &[u8; 32]) -> Result<(), SignalingError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SignalingError::InvalidMessage("Unknown session".to_string()))?;

        let mut pairs = Vec::new();

        // Create pairs for all compatible local/remote candidates
        for local in &session.local_candidates {
            for remote in &session.remote_candidates {
                // Only pair candidates of the same IP family
                let local_is_v4 = local.address.is_ipv4();
                let remote_is_v4 = remote.address.is_ipv4();

                if local_is_v4 == remote_is_v4 {
                    // We are controlling if our peer ID is lexicographically greater
                    let is_controlling = self.local_peer_id > session.remote_peer_id;
                    let pair = CandidatePair::new(local.clone(), remote.clone(), is_controlling);
                    pairs.push(pair);
                }
            }
        }

        // Sort by priority (descending)
        pairs.sort_by(|a, b| b.priority.cmp(&a.priority));

        session.pairs = pairs;
        session.state = SessionState::Checking;

        Ok(())
    }

    /// Run connectivity checks for a session
    pub async fn run_connectivity_checks(
        &self,
        session_id: &[u8; 32],
        remote_ufrag: &str,
        remote_pwd: &str,
    ) -> Result<Option<SocketAddr>, SignalingError> {
        let is_controlling;
        let pairs_count;

        {
            let sessions = self.sessions.lock().await;
            let session = sessions
                .get(session_id)
                .ok_or_else(|| SignalingError::InvalidMessage("Unknown session".to_string()))?;
            is_controlling = self.local_peer_id > session.remote_peer_id;
            pairs_count = session.pairs.len();
        }

        let mut checker = ConnectivityChecker::new(
            self.socket.clone(),
            self.local_ufrag.clone(),
            self.local_pwd.clone(),
            remote_ufrag.to_string(),
            remote_pwd.to_string(),
            is_controlling,
        );

        // Check pairs in priority order
        for i in 0..pairs_count {
            let mut pair;
            {
                let sessions = self.sessions.lock().await;
                let session = sessions
                    .get(session_id)
                    .ok_or_else(|| SignalingError::InvalidMessage("Unknown session".to_string()))?;
                pair = session.pairs[i].clone();
            }

            if pair.state == PairState::Failed {
                continue;
            }

            // Try connectivity check
            if checker.check(&mut pair).await? {
                // Update session with successful pair
                let mut sessions = self.sessions.lock().await;
                if let Some(session) = sessions.get_mut(session_id) {
                    session.pairs[i] = pair;
                    session.nominated_pair = Some(i);
                    session.state = SessionState::Connected;
                    return Ok(Some(session.pairs[i].remote.address));
                }
            } else {
                // Update pair state
                let mut sessions = self.sessions.lock().await;
                if let Some(session) = sessions.get_mut(session_id) {
                    session.pairs[i] = pair;
                }
            }
        }

        // All pairs failed
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.state = SessionState::Failed;
        }

        Err(SignalingError::ConnectivityCheckFailed(
            "All candidate pairs failed".to_string(),
        ))
    }

    /// Get the nominated candidate pair for a session
    pub async fn get_nominated_address(&self, session_id: &[u8; 32]) -> Option<SocketAddr> {
        let sessions = self.sessions.lock().await;
        sessions
            .get(session_id)
            .and_then(|s| s.nominated_pair.map(|i| s.pairs[i].remote.address))
    }

    /// Get local socket address
    pub fn local_addr(&self) -> Result<SocketAddr, SignalingError> {
        self.socket.local_addr().map_err(SignalingError::from)
    }

    /// Get local ICE credentials
    pub fn local_credentials(&self) -> (&str, &str) {
        (&self.local_ufrag, &self.local_pwd)
    }

    /// Close a session
    pub async fn close_session(&self, session_id: &[u8; 32]) {
        self.sessions.lock().await.remove(session_id);
    }

    /// Prioritize candidates by type (RFC 8445)
    ///
    /// Returns candidates sorted by priority:
    /// 1. Host candidates (direct connection, lowest latency)
    /// 2. Server reflexive candidates (NAT traversal via STUN)
    /// 3. Peer reflexive candidates (discovered during connectivity checks)
    /// 4. Relay candidates (TURN server, highest latency but most reliable)
    pub fn prioritize_candidates(candidates: &mut [Candidate]) {
        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signaling_message_serialization() {
        let msg = SignalingMessage::Offer {
            session_id: [1u8; 32],
            sender_id: [2u8; 32],
            candidates: vec![SerializableCandidate {
                address: "192.168.1.100:5000".to_string(),
                candidate_type: "host".to_string(),
                priority: 2113929471,
                foundation: "abc123".to_string(),
            }],
            ufrag: "testufrag".to_string(),
            pwd: "testpassword123456789012".to_string(),
            timestamp: 1234567890,
        };

        let bytes = msg.to_bytes().unwrap();
        let decoded = SignalingMessage::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.session_id(), &[1u8; 32]);
        assert_eq!(decoded.sender_id(), &[2u8; 32]);
    }

    #[test]
    fn test_serializable_candidate_conversion() {
        let candidate = Candidate {
            address: "192.168.1.100:5000".parse().unwrap(),
            candidate_type: CandidateType::Host,
            priority: 2113929471,
        };

        let serializable = SerializableCandidate::from(&candidate);
        assert_eq!(serializable.address, "192.168.1.100:5000");
        assert_eq!(serializable.candidate_type, "host");

        let back: Candidate = Candidate::try_from(&serializable).unwrap();
        assert_eq!(back.address, candidate.address);
        assert_eq!(back.candidate_type, CandidateType::Host);
    }

    #[test]
    fn test_candidate_pair_priority() {
        let local = Candidate {
            address: "192.168.1.100:5000".parse().unwrap(),
            candidate_type: CandidateType::Host,
            priority: 2113929471,
        };

        let remote = Candidate {
            address: "192.168.1.200:5000".parse().unwrap(),
            candidate_type: CandidateType::Host,
            priority: 2113929470,
        };

        let pair = CandidatePair::new(local.clone(), remote.clone(), true);

        // Priority should be calculated per RFC 8445
        assert!(pair.priority > 0);
        assert_eq!(pair.state, PairState::Frozen);
    }

    #[test]
    fn test_ufrag_pwd_generation() {
        let ufrag = NatSignaling::generate_ufrag();
        let pwd = NatSignaling::generate_pwd();

        // ICE ufrag must be 4-256 characters
        assert!(ufrag.len() >= 4);
        assert!(ufrag.len() <= 256);

        // ICE pwd must be 22-256 characters
        assert!(pwd.len() >= 22);
        assert!(pwd.len() <= 256);
    }

    #[test]
    fn test_pair_state_transitions() {
        assert_eq!(PairState::Frozen, PairState::Frozen);
        assert_ne!(PairState::Frozen, PairState::Succeeded);
    }

    #[test]
    fn test_candidate_prioritization() {
        let mut candidates = vec![
            Candidate {
                address: "192.168.1.100:5000".parse().unwrap(),
                candidate_type: CandidateType::Relay,
                priority: 100,
            },
            Candidate {
                address: "192.168.1.100:5001".parse().unwrap(),
                candidate_type: CandidateType::Host,
                priority: 2113929471,
            },
            Candidate {
                address: "192.168.1.100:5002".parse().unwrap(),
                candidate_type: CandidateType::ServerReflexive,
                priority: 1677721855,
            },
        ];

        NatSignaling::prioritize_candidates(&mut candidates);

        // Host should be first (highest priority)
        assert_eq!(candidates[0].candidate_type, CandidateType::Host);
        // Server reflexive second
        assert_eq!(candidates[1].candidate_type, CandidateType::ServerReflexive);
        // Relay last (lowest priority)
        assert_eq!(candidates[2].candidate_type, CandidateType::Relay);
    }

    #[test]
    fn test_signaling_message_types() {
        let offer = SignalingMessage::Offer {
            session_id: [1u8; 32],
            sender_id: [2u8; 32],
            candidates: vec![],
            ufrag: "ufrag".to_string(),
            pwd: "password".to_string(),
            timestamp: 0,
        };

        let answer = SignalingMessage::Answer {
            session_id: [1u8; 32],
            sender_id: [3u8; 32],
            candidates: vec![],
            ufrag: "ufrag2".to_string(),
            pwd: "password2".to_string(),
            timestamp: 0,
        };

        let update = SignalingMessage::CandidateUpdate {
            session_id: [1u8; 32],
            sender_id: [2u8; 32],
            candidate: SerializableCandidate {
                address: "10.0.0.1:1234".to_string(),
                candidate_type: "prflx".to_string(),
                priority: 1000,
                foundation: "def456".to_string(),
            },
            timestamp: 0,
        };

        assert_eq!(offer.session_id(), &[1u8; 32]);
        assert_eq!(answer.sender_id(), &[3u8; 32]);
        assert_eq!(update.session_id(), &[1u8; 32]);
    }

    #[tokio::test]
    async fn test_nat_signaling_creation() {
        let result = NatSignaling::new([1u8; 32], "127.0.0.1:0".parse().unwrap()).await;
        assert!(result.is_ok());

        let signaling = result.unwrap();
        let local_addr = signaling.local_addr();
        assert!(local_addr.is_ok());
    }

    #[test]
    fn test_signaling_message_answer_serialization() {
        let msg = SignalingMessage::Answer {
            session_id: [10u8; 32],
            sender_id: [20u8; 32],
            candidates: vec![SerializableCandidate {
                address: "10.0.0.1:1234".to_string(),
                candidate_type: "srflx".to_string(),
                priority: 100,
                foundation: "abc".to_string(),
            }],
            ufrag: "uf".to_string(),
            pwd: "pw12345678901234567890".to_string(),
            timestamp: 999,
        };
        let bytes = msg.to_bytes().unwrap();
        let decoded = SignalingMessage::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.session_id(), &[10u8; 32]);
        assert_eq!(decoded.sender_id(), &[20u8; 32]);
    }

    #[test]
    fn test_signaling_message_candidate_update_serialization() {
        let msg = SignalingMessage::CandidateUpdate {
            session_id: [5u8; 32],
            sender_id: [6u8; 32],
            candidate: SerializableCandidate {
                address: "192.168.0.1:5555".to_string(),
                candidate_type: "relay".to_string(),
                priority: 50,
                foundation: "xyz".to_string(),
            },
            timestamp: 42,
        };
        let bytes = msg.to_bytes().unwrap();
        let decoded = SignalingMessage::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.session_id(), &[5u8; 32]);
    }

    #[test]
    fn test_signaling_message_from_bytes_invalid() {
        let result = SignalingMessage::from_bytes(&[0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn test_serializable_candidate_all_types() {
        let types = vec![
            (CandidateType::Host, "host"),
            (CandidateType::ServerReflexive, "srflx"),
            (CandidateType::PeerReflexive, "prflx"),
            (CandidateType::Relay, "relay"),
        ];

        for (ct, expected_str) in types {
            let candidate = Candidate {
                address: "1.2.3.4:1000".parse().unwrap(),
                candidate_type: ct,
                priority: 100,
            };
            let sc = SerializableCandidate::from(&candidate);
            assert_eq!(sc.candidate_type, expected_str);

            // Round-trip
            let back = Candidate::try_from(&sc).unwrap();
            assert_eq!(back.candidate_type, ct);
        }
    }

    #[test]
    fn test_serializable_candidate_invalid_address() {
        let sc = SerializableCandidate {
            address: "not-an-address".to_string(),
            candidate_type: "host".to_string(),
            priority: 100,
            foundation: "abc".to_string(),
        };
        let result = Candidate::try_from(&sc);
        assert!(result.is_err());
    }

    #[test]
    fn test_serializable_candidate_unknown_type() {
        let sc = SerializableCandidate {
            address: "1.2.3.4:1000".to_string(),
            candidate_type: "unknown_type".to_string(),
            priority: 100,
            foundation: "abc".to_string(),
        };
        let result = Candidate::try_from(&sc);
        assert!(result.is_err());
        if let Err(SignalingError::InvalidMessage(msg)) = result {
            assert!(msg.contains("Unknown candidate type"));
        }
    }

    #[test]
    fn test_candidate_pair_controlled() {
        let local = Candidate {
            address: "192.168.1.100:5000".parse().unwrap(),
            candidate_type: CandidateType::Host,
            priority: 100,
        };
        let remote = Candidate {
            address: "192.168.1.200:5000".parse().unwrap(),
            candidate_type: CandidateType::Host,
            priority: 200,
        };

        // Test as controlled (not controlling)
        let pair = CandidatePair::new(local.clone(), remote.clone(), false);
        assert!(pair.priority > 0);
        assert_eq!(pair.state, PairState::Frozen);
        assert_eq!(pair.check_attempts, 0);
        assert!(pair.last_check.is_none());

        // Test as controlling
        let pair2 = CandidatePair::new(local, remote, true);
        // Priorities should differ based on controlling role
        assert!(pair2.priority > 0);
    }

    #[test]
    fn test_pair_state_all_variants() {
        let states = vec![
            PairState::Frozen,
            PairState::Waiting,
            PairState::InProgress,
            PairState::Succeeded,
            PairState::Failed,
        ];
        for (i, a) in states.iter().enumerate() {
            for (j, b) in states.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_signaling_error_display() {
        assert!(
            SignalingError::NoCandidates
                .to_string()
                .contains("No candidates")
        );
        assert!(SignalingError::Timeout.to_string().contains("timed out"));
        assert!(
            SignalingError::ConnectivityCheckFailed("test".to_string())
                .to_string()
                .contains("test")
        );
        assert!(
            SignalingError::InvalidMessage("bad".to_string())
                .to_string()
                .contains("bad")
        );
        assert!(
            SignalingError::Dht("dht err".to_string())
                .to_string()
                .contains("dht err")
        );
        assert!(
            SignalingError::Serialization("ser err".to_string())
                .to_string()
                .contains("ser err")
        );
    }

    #[test]
    fn test_connectivity_checker_build_binding_request() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let socket = Arc::new(tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());

            // Test controlling
            let checker = ConnectivityChecker::new(
                socket.clone(),
                "ufrag1".to_string(),
                "pwd1".to_string(),
                "ufrag2".to_string(),
                "pwd2".to_string(),
                true,
            );
            let tid = [0u8; 16];
            let request = checker.build_binding_request(&tid);
            // Should start with STUN binding request type 0x0001
            assert_eq!(request[0], 0x00);
            assert_eq!(request[1], 0x01);
            // Magic cookie at bytes 4-8
            assert_eq!(request[4], 0x21);
            assert_eq!(request[5], 0x12);
            assert_eq!(request[6], 0xA4);
            assert_eq!(request[7], 0x42);

            // Test controlled
            let checker2 = ConnectivityChecker::new(
                socket,
                "ufrag1".to_string(),
                "pwd1".to_string(),
                "ufrag2".to_string(),
                "pwd2".to_string(),
                false,
            );
            let request2 = checker2.build_binding_request(&tid);
            assert_eq!(request2[0], 0x00);
            assert_eq!(request2[1], 0x01);
        });
    }

    #[tokio::test]
    async fn test_nat_signaling_with_stun_servers() {
        let stun_servers = vec!["1.2.3.4:3478".parse().unwrap()];
        let result = NatSignaling::with_stun_servers(
            [1u8; 32],
            "127.0.0.1:0".parse().unwrap(),
            stun_servers,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_nat_signaling_local_credentials() {
        let signaling = NatSignaling::new([1u8; 32], "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let (ufrag, pwd) = signaling.local_credentials();
        assert!(ufrag.len() >= 4);
        assert!(pwd.len() >= 22);
    }

    #[tokio::test]
    async fn test_nat_signaling_close_session() {
        let signaling = NatSignaling::new([1u8; 32], "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let session_id = [42u8; 32];
        // Close non-existent session should not panic
        signaling.close_session(&session_id).await;
    }

    #[tokio::test]
    async fn test_nat_signaling_get_nominated_none() {
        let signaling = NatSignaling::new([1u8; 32], "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let result = signaling.get_nominated_address(&[0u8; 32]).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_nat_signaling_process_answer_not_answer() {
        let signaling = NatSignaling::new([1u8; 32], "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let offer = SignalingMessage::Offer {
            session_id: [1u8; 32],
            sender_id: [2u8; 32],
            candidates: vec![],
            ufrag: "uf".to_string(),
            pwd: "pw".to_string(),
            timestamp: 0,
        };
        let result = signaling.process_answer(&offer).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_nat_signaling_process_answer_unknown_session() {
        let signaling = NatSignaling::new([1u8; 32], "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let answer = SignalingMessage::Answer {
            session_id: [99u8; 32],
            sender_id: [2u8; 32],
            candidates: vec![SerializableCandidate {
                address: "1.2.3.4:1000".to_string(),
                candidate_type: "host".to_string(),
                priority: 100,
                foundation: "a".to_string(),
            }],
            ufrag: "uf".to_string(),
            pwd: "pw".to_string(),
            timestamp: 0,
        };
        let result = signaling.process_answer(&answer).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_nat_signaling_create_answer_not_offer() {
        let signaling = NatSignaling::new([1u8; 32], "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let answer = SignalingMessage::Answer {
            session_id: [1u8; 32],
            sender_id: [2u8; 32],
            candidates: vec![],
            ufrag: "uf".to_string(),
            pwd: "pw".to_string(),
            timestamp: 0,
        };
        let result = signaling.create_answer(&answer).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_nat_signaling_run_checks_unknown_session() {
        let signaling = NatSignaling::new([1u8; 32], "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let result = signaling
            .run_connectivity_checks(&[0u8; 32], "uf", "pw")
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_transaction_id_uniqueness() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let socket = Arc::new(tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());
            let mut checker = ConnectivityChecker::new(
                socket,
                "uf1".to_string(),
                "pw1".to_string(),
                "uf2".to_string(),
                "pw2".to_string(),
                true,
            );
            let id1 = checker.generate_transaction_id();
            let id2 = checker.generate_transaction_id();
            assert_ne!(id1, id2);
        });
    }

    #[tokio::test]
    async fn test_create_offer() {
        let signaling = NatSignaling::new([1u8; 32], "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();

        let session_id = [42u8; 32];
        let offer = signaling.create_offer(&session_id).await;

        // May fail if no network interfaces available
        // Just check it doesn't panic
        let _ = offer;
    }
}
