# Phase 9: v0.9.0 - Protocol Integration & Beta Release

**Target:** v0.9.0 Beta Release - Functional End-to-End Protocol
**Estimated Effort:** 85 Story Points (~4-5 weeks)
**Prerequisites:** Phase 8 (v0.8.0) complete - All components implemented and hardened

---

## Overview

Phase 9 transforms WRAITH Protocol from a collection of well-tested components into a fully integrated, working protocol capable of end-to-end file transfers. While v0.8.0 delivered all necessary building blocks (crypto, transport, obfuscation, discovery, file handling), they remain disconnected. Phase 9 creates the orchestration layer that ties everything together.

**What v0.9.0 Achieves:**
- **Working Protocol:** Can actually send and receive files end-to-end
- **CLI Functionality:** Commands work for real (not placeholders)
- **Integration Validated:** 7 ignored integration tests now passing
- **Performance Proven:** 4 benchmark targets met
- **Beta Quality:** Ready for early adopter testing

**Current Gap Analysis:**
- ✅ All components implemented (crypto, transport, obfuscation, discovery, files)
- ✅ All unit tests passing (962 active tests, Grade A+ quality)
- ❌ **Node API missing** - No orchestration layer to coordinate components
- ❌ **Integration incomplete** - Components work in isolation, not together
- ❌ **CLI non-functional** - Structured but contains placeholders
- ❌ **7 integration tests ignored** - Require full protocol integration
- ❌ **4 benchmarks removed** - Require end-to-end functionality

---

## Sprint 9.1: Node API & Core Integration (Weeks 1-2)

**Duration:** 2 weeks
**Story Points:** 34
**Goal:** Create the Node orchestration layer that coordinates all protocol components

### 9.1.1: Node API Design & Structure (8 SP)

**Objective:** Design and implement the `wraith_core::Node` struct that serves as the high-level protocol orchestrator.

```rust
// crates/wraith-core/src/node/mod.rs

use std::sync::Arc;
use tokio::sync::RwLock;
use wraith_crypto::{NoiseState, SessionCrypto, DoubleRatchet};
use wraith_transport::TransportFactory;
use wraith_obfuscation::{PaddingEngine, TimingObfuscator, ProtocolMimicry};
use wraith_discovery::DiscoveryManager;
use wraith_files::{FileChunker, FileReassembler, TransferSession};

/// High-level WRAITH protocol node
///
/// Coordinates all protocol layers:
/// - Cryptographic handshakes (Noise_XX)
/// - Transport selection (AF_XDP/UDP)
/// - Obfuscation (padding/timing/mimicry)
/// - Peer discovery (DHT/relay)
/// - File transfer (chunking/tree hashing)
pub struct Node {
    /// Node configuration
    config: NodeConfig,

    /// Identity keypair (Ed25519 + X25519)
    identity: Arc<Identity>,

    /// Transport layer (AF_XDP or UDP)
    transport: Arc<dyn Transport>,

    /// Discovery manager (DHT + NAT + relay)
    discovery: Arc<DiscoveryManager>,

    /// Active sessions (peer_id -> session)
    sessions: Arc<RwLock<HashMap<PeerId, Session>>>,

    /// Active transfers (transfer_id -> transfer state)
    transfers: Arc<RwLock<HashMap<TransferId, TransferSession>>>,

    /// Obfuscation configuration
    obfuscation: ObfuscationConfig,
}

impl Node {
    /// Create new node with random identity
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wraith_core::Node;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let node = Node::new_random().await?;
    ///     println!("Node ID: {}", node.node_id());
    ///     Ok(())
    /// }
    /// ```
    pub async fn new_random() -> Result<Self, NodeError>;

    /// Create node from existing identity
    pub async fn new_from_identity(identity: Identity) -> Result<Self, NodeError>;

    /// Create node with custom configuration
    pub async fn new_with_config(config: NodeConfig) -> Result<Self, NodeError>;

    /// Get node's public identity
    pub fn node_id(&self) -> &NodeId;

    /// Get node's public key
    pub fn public_key(&self) -> &PublicKey;
}

/// Node configuration
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Listen address
    pub listen_addr: SocketAddr,

    /// Enable AF_XDP (requires root)
    pub enable_xdp: bool,

    /// Transport settings
    pub transport: TransportConfig,

    /// Obfuscation settings
    pub obfuscation: ObfuscationConfig,

    /// Discovery settings
    pub discovery: DiscoveryConfig,

    /// Transfer settings
    pub transfer: TransferConfig,
}
```

**Tasks:**
- [ ] Create `crates/wraith-core/src/node/mod.rs` with Node struct
- [ ] Define NodeConfig with all subsystem settings
- [ ] Implement Node::new_random(), new_from_identity(), new_with_config()
- [ ] Create Identity wrapper (Ed25519 + X25519 keypairs)
- [ ] Add error types (NodeError)
- [ ] Write 5 tests (creation, config validation, identity management)

**Acceptance Criteria:**
- [ ] Node can be created with random or provided identity
- [ ] All configuration sections validated
- [ ] Node holds references to all subsystems
- [ ] Tests verify creation and basic operations
- [ ] API documented with examples

---

### 9.1.2: Session Management (13 SP)

**Objective:** Implement session lifecycle (establishment, maintenance, termination) with crypto integration.

```rust
// crates/wraith-core/src/node/session.rs

impl Node {
    /// Establish session with peer
    ///
    /// Workflow:
    /// 1. Lookup peer in DHT (if needed)
    /// 2. Attempt direct connection
    /// 3. Fall back to NAT traversal (hole punching)
    /// 4. Fall back to relay if needed
    /// 5. Perform Noise_XX handshake
    /// 6. Establish session crypto
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wraith_core::Node;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let node = Node::new_random().await?;
    /// # let peer_id = [0u8; 32];
    /// let session = node.establish_session(&peer_id).await?;
    /// println!("Session established: {}", session.session_id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn establish_session(
        &self,
        peer_id: &PeerId,
    ) -> Result<SessionId, SessionError>;

    /// Get existing session or establish new one
    pub async fn get_or_establish_session(
        &self,
        peer_id: &PeerId,
    ) -> Result<Arc<Session>, SessionError>;

    /// Close session
    pub async fn close_session(&self, session_id: &SessionId) -> Result<(), SessionError>;

    /// List active sessions
    pub async fn active_sessions(&self) -> Vec<SessionInfo>;
}

/// Session state
pub struct Session {
    session_id: SessionId,
    peer_id: PeerId,

    /// Noise handshake state
    noise: Arc<RwLock<NoiseState>>,

    /// Session crypto (AEAD + ratchet)
    crypto: Arc<SessionCrypto>,

    /// Transport connection
    connection: Arc<Connection>,

    /// Session state
    state: Arc<RwLock<SessionState>>,

    /// Statistics
    stats: Arc<RwLock<SessionStats>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Connecting,
    Handshaking,
    Established,
    Migrating,
    Closing,
    Closed,
}
```

**Implementation Details:**

**Connection Establishment Flow:**
```rust
async fn establish_connection(&self, peer_id: &PeerId) -> Result<Connection, SessionError> {
    // 1. Lookup peer address (DHT or cache)
    let peer_addr = self.lookup_peer(peer_id).await?;

    // 2. Attempt direct connection
    match self.transport.connect(peer_addr).await {
        Ok(conn) => return Ok(conn),
        Err(e) if e.is_connection_refused() => {
            // 3. Try NAT traversal
            if let Ok(conn) = self.nat_traversal(peer_id).await {
                return Ok(conn);
            }

            // 4. Fall back to relay
            self.relay_connect(peer_id).await
        }
        Err(e) => Err(e.into()),
    }
}
```

**Noise_XX Handshake Integration:**
```rust
async fn perform_handshake(&self, connection: &Connection) -> Result<SessionCrypto, SessionError> {
    // Initialize Noise_XX as initiator
    let mut noise = NoiseState::new_xx(
        Role::Initiator,
        self.identity.x25519_private_key().clone(),
        b"WRAITH-v1",
    );

    // Message 1: -> e
    let msg1 = noise.write_message_1(&[])?;
    connection.send_frame(Frame::handshake(msg1)).await?;

    // Message 2: <- e, ee, s, es
    let msg2_frame = connection.recv_frame().await?;
    let msg2_payload = noise.read_message_2(msg2_frame.payload())?;

    // Message 3: -> s, se
    let msg3 = noise.write_message_3(&[])?;
    connection.send_frame(Frame::handshake(msg3)).await?;

    // Extract session keys
    let (tx_key, rx_key) = noise.finalize()?;

    // Create session crypto
    Ok(SessionCrypto::new(tx_key, rx_key))
}
```

**Tasks:**
- [ ] Implement establish_session() with connection flow
- [ ] Integrate Noise_XX handshake (3-message pattern)
- [ ] Create Session struct with state machine
- [ ] Implement connection fallback (direct → NAT → relay)
- [ ] Add session timeout and keepalive
- [ ] Implement session cleanup on close
- [ ] Write 8 tests (establishment, handshake, fallback, timeout, close)

**Acceptance Criteria:**
- [ ] Sessions can be established with peers
- [ ] Noise_XX handshake completes successfully
- [ ] Connection fallback works (direct → NAT → relay)
- [ ] Sessions track state (connecting → established → closed)
- [ ] Timeouts and keepalives prevent stale sessions
- [ ] Tests cover all connection paths

---

### 9.1.3: Transport Layer Integration (8 SP)

**Objective:** Wire transport selection (AF_XDP vs UDP) into Node with configuration-based selection.

```rust
// crates/wraith-core/src/node/transport.rs

impl Node {
    /// Initialize transport layer based on configuration
    async fn init_transport(config: &NodeConfig) -> Result<Arc<dyn Transport>, TransportError> {
        if config.enable_xdp {
            // Try AF_XDP (requires root and capable NIC)
            match AfXdpTransport::new(&config.transport).await {
                Ok(transport) => {
                    tracing::info!("Using AF_XDP transport (zero-copy mode)");
                    return Ok(Arc::new(transport));
                }
                Err(e) => {
                    tracing::warn!("AF_XDP init failed: {}, falling back to UDP", e);
                }
            }
        }

        // Fallback: UDP transport
        let transport = UdpTransport::new(&config.transport).await?;
        tracing::info!("Using UDP transport (fallback mode)");
        Ok(Arc::new(transport))
    }

    /// Send frame to peer
    pub async fn send_frame(
        &self,
        session_id: &SessionId,
        frame: Frame,
    ) -> Result<(), TransportError>;

    /// Receive frame from any peer
    pub async fn recv_frame(&self) -> Result<(SessionId, Frame), TransportError>;
}
```

**Worker Pool Integration:**
```rust
// Start transport workers (thread-per-core model)
async fn start_transport_workers(&self) -> Result<(), TransportError> {
    let num_workers = num_cpus::get();

    for worker_id in 0..num_workers {
        let node = Arc::clone(&self.inner);
        let transport = Arc::clone(&self.transport);

        tokio::spawn(async move {
            // Pin to CPU core
            #[cfg(target_os = "linux")]
            worker_pool::pin_to_core(worker_id);

            loop {
                // Receive packets
                match transport.recv().await {
                    Ok((peer_addr, data)) => {
                        node.handle_packet(peer_addr, data).await;
                    }
                    Err(e) => {
                        tracing::error!("Transport recv error: {}", e);
                    }
                }
            }
        });
    }

    Ok(())
}
```

**Tasks:**
- [ ] Implement init_transport() with AF_XDP/UDP selection
- [ ] Add transport fallback logic with logging
- [ ] Integrate worker pool for packet processing
- [ ] Implement send_frame() and recv_frame()
- [ ] Add CPU pinning for workers (Linux)
- [ ] Write 4 tests (AF_XDP selection, UDP fallback, send/recv, worker pool)

**Acceptance Criteria:**
- [ ] AF_XDP used when available and configured
- [ ] Graceful fallback to UDP when AF_XDP unavailable
- [ ] Worker pool distributes packet processing
- [ ] send_frame() and recv_frame() functional
- [ ] Tests verify both transport modes

---

### 9.1.4: Basic File Transfer (Without Obfuscation) (5 SP)

**Objective:** Implement send_file() and receive_file() with minimal features (no obfuscation, single peer).

```rust
// crates/wraith-core/src/node/transfer.rs

impl Node {
    /// Send file to peer
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wraith_core::Node;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let node = Node::new_random().await?;
    /// # let peer_id = [0u8; 32];
    /// let transfer_id = node.send_file("document.pdf", &peer_id).await?;
    /// node.wait_for_transfer(transfer_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_file(
        &self,
        file_path: impl AsRef<Path>,
        peer_id: &PeerId,
    ) -> Result<TransferId, TransferError>;

    /// Wait for transfer to complete
    pub async fn wait_for_transfer(
        &self,
        transfer_id: TransferId,
    ) -> Result<(), TransferError>;

    /// Get transfer progress
    pub fn get_transfer_progress(
        &self,
        transfer_id: &TransferId,
    ) -> Option<TransferProgress>;
}

/// Transfer progress information
#[derive(Debug, Clone)]
pub struct TransferProgress {
    pub transferred_bytes: u64,
    pub total_bytes: u64,
    pub transferred_chunks: u64,
    pub total_chunks: u64,
    pub speed_bytes_per_sec: f64,
    pub eta_seconds: Option<u64>,
}
```

**Implementation:**
```rust
async fn send_file_impl(
    &self,
    file_path: impl AsRef<Path>,
    peer_id: &PeerId,
) -> Result<TransferId, TransferError> {
    // 1. Create transfer session
    let transfer_id = TransferId::new_random();
    let file_size = std::fs::metadata(&file_path)?.len();
    let mut session = TransferSession::new(
        transfer_id,
        Direction::Send,
        file_path.as_ref(),
        file_size,
        self.config.transfer.chunk_size,
    );

    // 2. Chunk file
    let mut chunker = FileChunker::new(&file_path, self.config.transfer.chunk_size)?;

    // 3. Compute tree hash
    let tree_hash = compute_tree_hash(&file_path, self.config.transfer.chunk_size)?;

    // 4. Establish session with peer
    let session_id = self.establish_session(peer_id).await?;
    let peer_session = self.sessions.read().await.get(&session_id).cloned()
        .ok_or(TransferError::SessionNotFound)?;

    // 5. Send transfer initiation
    let init_frame = Frame::transfer_init(transfer_id, file_size, tree_hash.root());
    peer_session.send_frame(init_frame).await?;

    // 6. Send chunks
    while let Some(chunk_data) = chunker.read_chunk()? {
        let chunk_index = chunker.current_chunk_index();
        let data_frame = Frame::data(chunk_index, chunk_data);
        peer_session.send_frame(data_frame).await?;

        session.mark_chunk_transferred(chunk_index);
    }

    // 7. Store transfer session
    self.transfers.write().await.insert(transfer_id, session);

    Ok(transfer_id)
}
```

**Tasks:**
- [ ] Implement send_file() with chunking and tree hash
- [ ] Implement receive_file() handler
- [ ] Create TransferProgress tracking
- [ ] Implement wait_for_transfer() with polling
- [ ] Add transfer state management
- [ ] Write 6 tests (send, receive, progress, completion, errors)

**Acceptance Criteria:**
- [ ] Can send file end-to-end (single peer, no obfuscation)
- [ ] File chunking works correctly
- [ ] Tree hash computed and verified
- [ ] Progress tracking accurate
- [ ] Tests verify complete transfer workflow

---

## Sprint 9.2: Discovery & NAT Integration (Week 3)

**Duration:** 1 week
**Story Points:** 21
**Goal:** Wire DHT, NAT traversal, and relay into session establishment

### 9.2.1: DHT Integration (8 SP)

**Objective:** Integrate DiscoveryManager for peer lookup and announcements.

```rust
// crates/wraith-core/src/node/discovery.rs

impl Node {
    /// Lookup peer in DHT
    async fn lookup_peer(&self, peer_id: &PeerId) -> Result<PeerInfo, DiscoveryError> {
        // 1. Check local cache
        if let Some(info) = self.peer_cache.get(peer_id).await {
            return Ok(info);
        }

        // 2. Query DHT
        let info_hash = self.discovery.compute_info_hash(peer_id);
        let peers = self.discovery.find_value(&info_hash).await?;

        // 3. Select best peer (lowest latency)
        let peer_info = peers.into_iter()
            .min_by_key(|p| p.latency)
            .ok_or(DiscoveryError::PeerNotFound)?;

        // 4. Cache result
        self.peer_cache.insert(*peer_id, peer_info.clone()).await;

        Ok(peer_info)
    }

    /// Announce self to DHT
    pub async fn announce(&self) -> Result<(), DiscoveryError> {
        let info_hash = self.discovery.compute_info_hash(&self.node_id());
        let peer_info = PeerInfo {
            peer_id: self.node_id(),
            addrs: self.listen_addrs(),
            public_key: self.public_key().clone(),
        };

        self.discovery.announce(&info_hash, peer_info).await
    }

    /// Start periodic DHT announcements
    pub async fn start_announcements(&self) {
        let node = Arc::clone(&self.inner);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 min

            loop {
                interval.tick().await;

                if let Err(e) = node.announce().await {
                    tracing::warn!("DHT announcement failed: {}", e);
                }
            }
        });
    }
}
```

**Tasks:**
- [ ] Implement lookup_peer() with DHT queries
- [ ] Add peer cache with TTL
- [ ] Implement announce() to DHT
- [ ] Add periodic announcement task
- [ ] Integrate info_hash computation with privacy enhancement
- [ ] Write 5 tests (lookup, announce, cache, periodic updates)

**Acceptance Criteria:**
- [ ] Peers can be discovered via DHT
- [ ] Announcements propagate to DHT
- [ ] Peer cache reduces DHT queries
- [ ] Periodic announcements maintain presence
- [ ] Tests verify DHT integration

---

### 9.2.2: NAT Traversal Integration (8 SP)

**Objective:** Implement hole punching and relay fallback in session establishment.

```rust
// crates/wraith-core/src/node/nat.rs

impl Node {
    /// Attempt NAT traversal
    async fn nat_traversal(&self, peer_id: &PeerId) -> Result<Connection, SessionError> {
        // 1. Detect NAT type
        let nat_type = self.discovery.detect_nat_type().await?;

        match nat_type {
            NatType::FullCone | NatType::RestrictedCone | NatType::PortRestrictedCone => {
                // Hole punching likely to work
                self.attempt_hole_punch(peer_id).await
            }
            NatType::Symmetric => {
                // Birthday attack (low success rate)
                self.attempt_birthday_attack(peer_id).await
                    .or_else(|_| self.relay_connect(peer_id).await)
            }
            NatType::Open => {
                // No NAT, direct connection should have worked
                Err(SessionError::ConnectionFailed)
            }
        }
    }

    /// UDP hole punching
    async fn attempt_hole_punch(&self, peer_id: &PeerId) -> Result<Connection, SessionError> {
        // 1. Get peer's public endpoint from STUN
        let peer_endpoint = self.discovery.get_peer_endpoint(peer_id).await?;

        // 2. Send simultaneous packets
        let local_endpoint = self.transport.local_addr()?;

        // Send from our side
        self.transport.send_to(&peer_endpoint, b"PUNCH").await?;

        // Wait for peer's packet (opens hole)
        tokio::time::timeout(
            Duration::from_secs(5),
            self.transport.recv_from(&local_endpoint)
        ).await??;

        // 3. Establish connection
        self.transport.connect(peer_endpoint).await
    }

    /// Relay connection as last resort
    async fn relay_connect(&self, peer_id: &PeerId) -> Result<Connection, SessionError> {
        // 1. Select relay server
        let relay = self.discovery.select_relay().await?;

        // 2. Connect to relay
        let relay_conn = self.transport.connect(relay.addr()).await?;

        // 3. Request relay to peer
        relay_conn.send_frame(Frame::relay_request(peer_id)).await?;

        // 4. Wait for relay confirmation
        let response = relay_conn.recv_frame().await?;

        if response.is_relay_ok() {
            Ok(relay_conn)
        } else {
            Err(SessionError::RelayFailed)
        }
    }
}
```

**Tasks:**
- [ ] Implement nat_traversal() with type detection
- [ ] Implement attempt_hole_punch() with simultaneous send
- [ ] Implement relay_connect() as fallback
- [ ] Add NAT type caching
- [ ] Integrate STUN for endpoint discovery
- [ ] Write 6 tests (hole punch success/fail, relay, NAT detection)

**Acceptance Criteria:**
- [ ] Hole punching works for non-symmetric NAT
- [ ] Relay fallback works when hole punching fails
- [ ] NAT type detected correctly
- [ ] Tests cover all NAT scenarios

---

### 9.2.3: Connection Management (5 SP)

**Objective:** Implement connection state tracking, migration, and timeout handling.

```rust
// crates/wraith-core/src/node/connection.rs

impl Node {
    /// Handle connection migration (IP change)
    pub async fn migrate_connection(
        &self,
        session_id: &SessionId,
        new_addr: SocketAddr,
    ) -> Result<(), SessionError> {
        let session = self.sessions.write().await
            .get_mut(session_id)
            .ok_or(SessionError::SessionNotFound)?;

        // Send PATH_CHALLENGE
        let challenge = random_bytes(32);
        session.send_frame(Frame::path_challenge(challenge.clone())).await?;

        // Wait for PATH_RESPONSE
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            session.recv_frame_matching(|f| f.is_path_response())
        ).await??;

        // Verify challenge
        if response.path_response_data() == challenge {
            session.migrate_to(new_addr).await?;
            tracing::info!("Connection migrated to {}", new_addr);
            Ok(())
        } else {
            Err(SessionError::MigrationFailed)
        }
    }

    /// Handle session timeouts
    async fn monitor_sessions(&self) {
        let node = Arc::clone(&self.inner);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let mut sessions = node.sessions.write().await;
                let timeout = Duration::from_secs(180); // 3 min idle

                sessions.retain(|session_id, session| {
                    if session.idle_time() > timeout {
                        tracing::info!("Session {} timed out", session_id);
                        false
                    } else {
                        true
                    }
                });
            }
        });
    }
}
```

**Tasks:**
- [ ] Implement migrate_connection() with PATH_CHALLENGE/RESPONSE
- [ ] Add session timeout monitoring
- [ ] Implement connection health checks (PING/PONG)
- [ ] Add automatic reconnection for dropped connections
- [ ] Write 4 tests (migration, timeout, health check, reconnect)

**Acceptance Criteria:**
- [ ] Connection migration works on IP change
- [ ] Stale sessions cleaned up after timeout
- [ ] Health checks detect dead connections
- [ ] Tests verify all connection states

---

## Sprint 9.3: Obfuscation Integration (Week 3.5-4)

**Duration:** 0.5 weeks
**Story Points:** 13
**Goal:** Wire padding, timing, and protocol mimicry into packet transmission

### 9.3.1: Padding & Timing Integration (8 SP)

**Objective:** Apply obfuscation to all outgoing packets based on configuration.

```rust
// crates/wraith-core/src/node/obfuscation.rs

impl Node {
    /// Apply obfuscation to frame before sending
    async fn obfuscate_frame(&self, frame: Frame) -> Result<Vec<u8>, ObfuscationError> {
        let mut data = frame.encode()?;

        // 1. Apply padding
        let target_size = self.obfuscation.padding.padded_size(data.len());
        self.obfuscation.padding.pad(&mut data, target_size);

        // 2. Apply timing delay
        if let Some(delay) = self.obfuscation.timing.next_delay() {
            tokio::time::sleep(delay).await;
        }

        Ok(data)
    }

    /// Initialize obfuscation engines based on config
    fn init_obfuscation(config: &ObfuscationConfig) -> ObfuscationEngines {
        let padding = PaddingEngine::new(config.padding_mode);
        let timing = TimingObfuscator::new(config.timing_mode);

        ObfuscationEngines { padding, timing }
    }
}

struct ObfuscationEngines {
    padding: PaddingEngine,
    timing: TimingObfuscator,
}
```

**Tasks:**
- [ ] Implement obfuscate_frame() with padding and timing
- [ ] Add configuration-based mode selection
- [ ] Integrate PaddingEngine from wraith-obfuscation
- [ ] Integrate TimingObfuscator from wraith-obfuscation
- [ ] Write 5 tests (padding modes, timing modes, configuration)

**Acceptance Criteria:**
- [ ] Padding applied to all outgoing frames
- [ ] Timing delays based on configured distribution
- [ ] Modes selectable via configuration
- [ ] Tests verify obfuscation behavior

---

### 9.3.2: Protocol Mimicry (5 SP)

**Objective:** Wrap packets in TLS/WebSocket/DoH based on configuration.

```rust
// crates/wraith-core/src/node/mimicry.rs

impl Node {
    /// Wrap frame in protocol mimicry
    async fn apply_mimicry(&self, data: Vec<u8>) -> Result<Vec<u8>, ObfuscationError> {
        match self.config.obfuscation.mimicry_mode {
            MimicryMode::None => Ok(data),
            MimicryMode::Tls => {
                TlsMimicry::wrap_application_data(&data)
            }
            MimicryMode::WebSocket => {
                WebSocketMimicry::wrap_binary_frame(&data)
            }
            MimicryMode::DoH => {
                DohMimicry::wrap_dns_query(&data)
            }
        }
    }

    /// Unwrap protocol mimicry
    async fn unwrap_mimicry(&self, data: Vec<u8>) -> Result<Vec<u8>, ObfuscationError> {
        // Try all modes (mimicry is self-describing)
        if let Ok(unwrapped) = TlsMimicry::unwrap(&data) {
            return Ok(unwrapped);
        }

        if let Ok(unwrapped) = WebSocketMimicry::unwrap(&data) {
            return Ok(unwrapped);
        }

        if let Ok(unwrapped) = DohMimicry::unwrap(&data) {
            return Ok(unwrapped);
        }

        // No mimicry, return as-is
        Ok(data)
    }
}
```

**Tasks:**
- [ ] Implement apply_mimicry() with mode selection
- [ ] Implement unwrap_mimicry() with auto-detection
- [ ] Integrate TLS/WebSocket/DoH wrappers from wraith-obfuscation
- [ ] Add mimicry mode configuration
- [ ] Write 4 tests (each mode + none)

**Acceptance Criteria:**
- [ ] TLS mimicry generates valid-looking records
- [ ] WebSocket mimicry generates valid frames
- [ ] DoH mimicry generates valid DNS queries
- [ ] Auto-unwrap detects mode correctly
- [ ] Tests verify all modes

---

## Sprint 9.4: File Transfer Engine & Testing (Week 4-5)

**Duration:** 1-1.5 weeks
**Story Points:** 17
**Goal:** Complete multi-peer downloads, implement ignored tests, and validate performance

### 9.4.1: Multi-Peer Download Coordination (8 SP)

**Objective:** Implement parallel downloads from multiple peers with chunk assignment.

```rust
// crates/wraith-core/src/node/multi_peer.rs

impl Node {
    /// Start multi-peer download
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wraith_core::Node;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let node = Node::new_random().await?;
    /// # let peers = vec![[0u8; 32], [1u8; 32]];
    /// let transfer_id = node.start_multi_peer_download(
    ///     "output.zip",
    ///     &peers,
    ///     Some(tree_hash),
    /// ).await?;
    /// node.wait_for_transfer(transfer_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start_multi_peer_download(
        &self,
        output_path: impl AsRef<Path>,
        peers: &[PeerId],
        tree_hash: Option<FileTreeHash>,
    ) -> Result<TransferId, TransferError>;

    /// Coordinate chunk assignment across peers
    async fn assign_chunks_to_peers(
        &self,
        transfer_id: &TransferId,
    ) -> Result<(), TransferError> {
        let mut transfer = self.transfers.write().await
            .get_mut(transfer_id)
            .ok_or(TransferError::TransferNotFound)?;

        let missing = transfer.missing_chunks();
        let peers = transfer.active_peers();

        if peers.is_empty() {
            return Err(TransferError::NoPeersAvailable);
        }

        // Distribute chunks evenly
        for (i, chunk_index) in missing.enumerate() {
            let peer_index = i % peers.len();
            let peer = &peers[peer_index];

            // Request chunk from peer
            self.request_chunk(transfer_id, peer, chunk_index).await?;
        }

        Ok(())
    }

    /// Request specific chunk from peer
    async fn request_chunk(
        &self,
        transfer_id: &TransferId,
        peer_id: &PeerId,
        chunk_index: u64,
    ) -> Result<(), TransferError>;
}
```

**Tasks:**
- [ ] Implement start_multi_peer_download()
- [ ] Implement assign_chunks_to_peers() with even distribution
- [ ] Add peer health tracking (bytes transferred, last activity)
- [ ] Implement automatic peer removal on timeout
- [ ] Add chunk request/response handling
- [ ] Write 6 tests (2 peers, 5 peers, peer failure, rebalancing)

**Acceptance Criteria:**
- [ ] Can download from multiple peers simultaneously
- [ ] Chunks distributed evenly across peers
- [ ] Failed peers removed and chunks reassigned
- [ ] Tests verify linear speedup up to 5 peers
- [ ] No duplicate chunk downloads

---

### 9.4.2: Integration Tests Implementation (5 SP)

**Objective:** Implement the 7 integration tests currently ignored in tests/integration_tests.rs.

```rust
// tests/integration_tests.rs

#[tokio::test]
async fn test_end_to_end_transfer() -> Result<()> {
    // Create sender and receiver nodes
    let sender = Node::new_random().await?;
    let receiver = Node::new_random().await?;

    // Create test file (5 MB)
    let test_file = create_test_file(5 * 1024 * 1024)?;
    let tree_hash = compute_tree_hash(&test_file, DEFAULT_CHUNK_SIZE)?;

    // Start transfer
    let transfer_id = sender.send_file(&test_file, receiver.node_id()).await?;

    // Wait for completion
    sender.wait_for_transfer(transfer_id).await?;

    // Verify integrity
    let received_file = receiver.received_files().pop().unwrap();
    let received_hash = compute_tree_hash(&received_file, DEFAULT_CHUNK_SIZE)?;

    assert_eq!(tree_hash.root(), received_hash.root());
    Ok(())
}

#[tokio::test]
async fn test_connection_establishment() -> Result<()> {
    let node1 = Node::new_random().await?;
    let node2 = Node::new_random().await?;

    // Establish session
    let session_id = node1.establish_session(node2.node_id()).await?;

    // Verify session active
    assert!(node1.active_sessions().await.iter().any(|s| s.session_id == session_id));

    Ok(())
}

#[tokio::test]
async fn test_obfuscation_integration() -> Result<()> {
    // Test with high obfuscation
    let mut config = NodeConfig::default();
    config.obfuscation.padding_mode = PaddingMode::ConstantRate;
    config.obfuscation.timing_mode = TimingMode::Normal { mean: 100, stddev: 20 };
    config.obfuscation.mimicry_mode = MimicryMode::Tls;

    let node = Node::new_with_config(config).await?;

    // Send file
    let test_file = create_test_file(1024 * 1024)?;
    let transfer_id = node.send_file(&test_file, node.node_id()).await?;

    // Verify obfuscation applied (check packet sizes, timing)
    // ... implementation ...

    Ok(())
}

// ... implement remaining 4 tests ...
```

**Tasks:**
- [ ] Implement test_end_to_end_transfer (5MB file)
- [ ] Implement test_connection_establishment (Noise_XX handshake)
- [ ] Implement test_obfuscation_integration (padding, timing, mimicry)
- [ ] Implement test_discovery_integration (DHT lookup)
- [ ] Implement test_multi_path_transfer (connection migration)
- [ ] Implement test_error_recovery (packet loss, retransmission)
- [ ] Implement test_concurrent_transfers (multiple simultaneous)

**Acceptance Criteria:**
- [ ] All 7 integration tests pass
- [ ] Tests cover end-to-end workflows
- [ ] Tests validate performance targets
- [ ] No tests ignored

---

### 9.4.3: Performance Benchmarks (4 SP)

**Objective:** Implement the 4 benchmarks removed in Phase 7 (benches/transfer.rs).

```rust
// benches/transfer.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use wraith_core::Node;

fn bench_transfer_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("transfer_throughput");
    let rt = tokio::runtime::Runtime::new().unwrap();

    for size in [1_000_000, 10_000_000, 100_000_000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let sender = Node::new_random().await.unwrap();
                let receiver = Node::new_random().await.unwrap();

                let test_file = create_test_file(size);

                let start = std::time::Instant::now();
                let transfer_id = sender.send_file(&test_file, receiver.node_id()).await.unwrap();
                sender.wait_for_transfer(transfer_id).await.unwrap();
                let elapsed = start.elapsed();

                let throughput_mbps = (size as f64 * 8.0) / elapsed.as_secs_f64() / 1_000_000.0;

                black_box(throughput_mbps)
            });
        });
    }

    group.finish();
}

// Target: >300 Mbps on 1 Gbps LAN
fn bench_transfer_latency(c: &mut Criterion) { /* ... */ }

// Target: >95% link utilization
fn bench_bbr_utilization(c: &mut Criterion) { /* ... */ }

// Target: Linear speedup to 5 peers
fn bench_multi_peer_speedup(c: &mut Criterion) { /* ... */ }

criterion_group!(
    benches,
    bench_transfer_throughput,
    bench_transfer_latency,
    bench_bbr_utilization,
    bench_multi_peer_speedup
);
criterion_main!(benches);
```

**Performance Targets:**
- **Throughput:** >300 Mbps on 1 Gbps LAN
- **Latency:** <10ms RTT on LAN
- **BBR Utilization:** >95% link utilization
- **Multi-Peer Speedup:** ~Linear to 5 peers

**Tasks:**
- [ ] Implement bench_transfer_throughput (1MB, 10MB, 100MB)
- [ ] Implement bench_transfer_latency (handshake, RTT, first byte)
- [ ] Implement bench_bbr_utilization (1GB transfer, bandwidth tracking)
- [ ] Implement bench_multi_peer_speedup (1-5 peers)
- [ ] Document benchmark results in README

**Acceptance Criteria:**
- [ ] All benchmarks run successfully
- [ ] Performance targets met or documented why not
- [ ] Benchmarks automated in CI (optional)
- [ ] Results published in documentation

---

## Definition of Done (Phase 9)

### Functionality
- [ ] Node API complete and documented
- [ ] Sessions establish successfully (Noise_XX handshake)
- [ ] Files transfer end-to-end (single peer)
- [ ] Multi-peer downloads work (2-5 peers)
- [ ] DHT discovery functional
- [ ] NAT traversal works (hole punching + relay fallback)
- [ ] Obfuscation applied (padding, timing, mimicry)
- [ ] CLI commands functional (send, receive, status)

### Testing
- [ ] All 7 integration tests passing (no ignored tests)
- [ ] All 4 performance benchmarks implemented and passing targets
- [ ] Unit tests for Node API (20+ tests)
- [ ] End-to-end transfer verified (5MB file)

### Quality
- [ ] All tests passing (target: 1000+ tests total)
- [ ] cargo clippy clean (zero warnings)
- [ ] cargo fmt clean
- [ ] Documentation complete (rustdoc for Node API)
- [ ] Examples in documentation work

### Performance
- [ ] Throughput: >300 Mbps on 1 Gbps LAN
- [ ] Latency: <10ms RTT on LAN
- [ ] BBR utilization: >95%
- [ ] Multi-peer speedup: Linear to 5 peers

---

## Success Metrics

### Technical Metrics
- [ ] Node API has 15+ public methods
- [ ] 7 integration tests pass (currently ignored)
- [ ] 4 benchmarks meet performance targets
- [ ] Test count >1000 (current: 973)
- [ ] Code coverage maintained >80%

### Functional Metrics
- [ ] Can send 1GB file in <10 seconds (1 Gbps LAN)
- [ ] Can download from 5 peers simultaneously
- [ ] NAT traversal success rate >90%
- [ ] Resume works after interruption

### Quality Metrics
- [ ] Zero clippy warnings
- [ ] Zero compilation warnings
- [ ] Technical debt ratio maintained <15%
- [ ] Grade A quality (maintained)

---

## Risk Management

### High-Risk Areas

**1. Node API Complexity**
- **Risk:** Coordinating 5 subsystems is complex
- **Mitigation:** Incremental development, thorough testing
- **Contingency:** Simplify API, defer advanced features to v1.0.0

**2. Performance Targets**
- **Risk:** May not achieve 300+ Mbps throughput
- **Mitigation:** Profile early, optimize hot paths
- **Contingency:** Document actual performance, optimize in v1.0.0

**3. Multi-Peer Coordination**
- **Risk:** Chunk assignment algorithms may be inefficient
- **Mitigation:** Simple round-robin initially, optimize if needed
- **Contingency:** Single-peer transfers work, multi-peer optional

---

## Dependencies & Blockers

### External Dependencies
- Phase 8 (v0.8.0) complete
- All crates functional
- tokio async runtime

### Potential Blockers
1. **Integration Complexity:** Components may not integrate cleanly
   - **Mitigation:** Design Node API carefully, incremental testing

2. **Performance Issues:** Benchmarks may reveal bottlenecks
   - **Mitigation:** Profile and optimize, defer some optimizations to v1.0.0

---

## Completion Checklist

- [ ] Sprint 9.1: Node API & Core Integration (34 SP)
- [ ] Sprint 9.2: Discovery & NAT Integration (21 SP)
- [ ] Sprint 9.3: Obfuscation Integration (13 SP)
- [ ] Sprint 9.4: File Transfer Engine & Testing (17 SP)
- [ ] All acceptance criteria met
- [ ] All integration tests passing
- [ ] All benchmarks implemented
- [ ] Documentation updated
- [ ] README updated (version, features, status)
- [ ] CHANGELOG.md updated
- [ ] Release v0.9.0 prepared

**Estimated Completion:** 4-5 weeks

---

**WRAITH Protocol v0.9.0 - BETA RELEASE READY!**

After Phase 9, WRAITH Protocol will be a **working, integrated protocol** capable of end-to-end secure file transfers with obfuscation and multi-peer downloads. Ready for early adopter testing and feedback.
