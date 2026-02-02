# WRAITH Protocol Integration Guide

**Version:** 1.0.0
**Last Updated:** 2025-12-05
**Status:** Complete Integration Reference

---

## Table of Contents

1. [Library Integration](#1-library-integration)
2. [Protocol Integration](#2-protocol-integration)
3. [Transport Integration](#3-transport-integration)
4. [Discovery Integration](#4-discovery-integration)
5. [Error Handling](#5-error-handling)

---

## 1. Library Integration

### 1.1 Adding WRAITH to Your Project

**Cargo Dependencies:**

Add WRAITH crates to your `Cargo.toml`:

```toml
[dependencies]
# Core protocol orchestration (Node API)
wraith-core = "0.9"

# Optional: Specific crates for fine-grained control
wraith-crypto = "0.9"      # Cryptographic primitives
wraith-transport = "0.9"   # Network transport layer
wraith-obfuscation = "0.9" # Traffic obfuscation
wraith-discovery = "0.9"   # DHT and NAT traversal
wraith-files = "0.9"       # File I/O and chunking

# Async runtime (required)
tokio = { version = "1", features = ["full"] }

# Logging (recommended)
tracing = "0.1"
tracing-subscriber = "0.3"
```

**Feature Flags:**

```toml
[dependencies.wraith-core]
version = "0.9"
features = [
    "af-xdp",      # Enable AF_XDP kernel bypass (Linux only)
    "io-uring",    # Enable io_uring file I/O (Linux only)
    "simd",        # Enable SIMD acceleration (default)
    "compression", # Enable compression support
]
```

### 1.2 Node API Quick Start

The Node API provides a high-level interface for all WRAITH operations.

**Basic Example:**

```rust
use wraith_core::node::{Node, NodeConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create node with default configuration
    let config = NodeConfig::default();
    let mut node = Node::new_random(config)?;

    // Start the node
    node.start().await?;
    println!("Node started: {}", node.id());

    // Send a file to a peer
    let peer_id = "a1b2c3d4e5f67890...".parse()?;
    let file_path = PathBuf::from("document.pdf");
    let transfer_id = node.send_file(peer_id, file_path).await?;

    // Wait for transfer to complete
    node.wait_for_transfer(transfer_id).await?;
    println!("Transfer complete!");

    // Stop the node gracefully
    node.stop().await?;

    Ok(())
}
```

### 1.3 Node Configuration

**Complete Configuration Example:**

```rust
use wraith_core::node::{
    NodeConfig, TransportConfig, ObfuscationConfig,
    DiscoveryConfig, TransferConfig, LoggingConfig,
};
use std::time::Duration;

let config = NodeConfig {
    // Transport settings
    transport: TransportConfig {
        af_xdp_enabled: true,
        io_uring_enabled: true,
        udp_buffer_size: 2 * 1024 * 1024, // 2 MB
        worker_threads: 4,
        connection_timeout: Duration::from_secs(30),
        idle_timeout: Duration::from_secs(60),
    },

    // Obfuscation settings
    obfuscation: ObfuscationConfig {
        padding_mode: PaddingMode::SizeClasses,
        timing_mode: TimingMode::Uniform { min_ms: 0, max_ms: 50 },
        protocol_mimicry: ProtocolMimicry::None,
    },

    // Discovery settings
    discovery: DiscoveryConfig {
        dht_enabled: true,
        bootstrap_nodes: vec![
            "bootstrap1.wraith.network:41641".parse()?,
            "bootstrap2.wraith.network:41641".parse()?,
        ],
        nat_traversal_enabled: true,
        relay_enabled: true,
        relay_servers: vec![
            "relay1.wraith.network:41641".parse()?,
        ],
        announcement_interval: Duration::from_secs(1800), // 30 min
    },

    // Transfer settings
    transfer: TransferConfig {
        chunk_size: 256 * 1024, // 256 KB
        max_concurrent_transfers: 10,
        max_concurrent_chunks: 16,
        download_dir: PathBuf::from("~/Downloads/wraith"),
        resume_enabled: true,
        multi_peer_enabled: true,
        max_peers_per_download: 5,
    },

    // Logging settings
    logging: LoggingConfig {
        level: LogLevel::Info,
        metrics_enabled: true,
    },
};

let node = Node::new_random(config)?;
```

### 1.4 Session Management

**Establish Session with Peer:**

```rust
use wraith_core::node::Node;

// Establish session (performs Noise_XX handshake)
let peer_id = "a1b2c3d4e5f67890...".parse()?;
let session_id = node.establish_session(peer_id).await?;
println!("Session established: {}", session_id);

// Get or establish session (reuses existing if available)
let session_id = node.get_or_establish_session(peer_id).await?;

// Close session
node.close_session(peer_id).await?;

// List active sessions
let sessions = node.active_sessions().await;
for (peer_id, session_info) in sessions {
    println!("Peer: {}, RTT: {}ms, Bytes: {}",
             peer_id, session_info.rtt_ms, session_info.bytes_sent);
}
```

**Session Events:**

```rust
use wraith_core::node::SessionEvent;

// Subscribe to session events
let mut events = node.subscribe_session_events();

tokio::spawn(async move {
    while let Some(event) = events.recv().await {
        match event {
            SessionEvent::Established { peer_id, session_id } => {
                println!("Session established with {}", peer_id);
            }
            SessionEvent::Closed { peer_id, reason } => {
                println!("Session closed with {}: {:?}", peer_id, reason);
            }
            SessionEvent::Error { peer_id, error } => {
                eprintln!("Session error with {}: {}", peer_id, error);
            }
        }
    }
});
```

### 1.5 File Transfer API

**Sending Files:**

```rust
use wraith_core::node::{Node, SendOptions};
use std::path::PathBuf;

let peer_id = "a1b2c3d4e5f67890...".parse()?;
let file_path = PathBuf::from("large_file.zip");

// Send with options
let options = SendOptions {
    obfuscation_level: ObfuscationLevel::High,
    chunk_size: Some(512 * 1024), // 512 KB
    resume: true,
};

let transfer_id = node.send_file_with_options(peer_id, file_path, options).await?;

// Monitor progress
loop {
    let progress = node.get_transfer_progress(transfer_id).await?;
    println!("Progress: {:.1}%", progress.percentage);

    if progress.completed {
        println!("Transfer complete!");
        break;
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

**Receiving Files:**

```rust
use wraith_core::node::{Node, ReceiveCallback};

// Set up receive callback
node.set_receive_callback(|transfer_info| async move {
    println!("Incoming transfer from {}:", transfer_info.peer_id);
    println!("  File: {}", transfer_info.filename);
    println!("  Size: {} bytes", transfer_info.file_size);
    println!("  Hash: {}", transfer_info.root_hash);

    // Accept or reject transfer
    if transfer_info.file_size < 100 * 1024 * 1024 { // Accept files < 100 MB
        Ok(true)
    } else {
        Ok(false)
    }
}).await?;

// Or auto-accept all transfers
node.set_auto_accept(true).await?;
```

**Transfer Events:**

```rust
use wraith_core::node::TransferEvent;

let mut events = node.subscribe_transfer_events();

tokio::spawn(async move {
    while let Some(event) = events.recv().await {
        match event {
            TransferEvent::Started { transfer_id, peer_id, filename } => {
                println!("Transfer started: {} from {}", filename, peer_id);
            }
            TransferEvent::Progress { transfer_id, bytes_transferred, total_bytes } => {
                let pct = (bytes_transferred as f64 / total_bytes as f64) * 100.0;
                println!("Progress: {:.1}%", pct);
            }
            TransferEvent::Completed { transfer_id, duration } => {
                println!("Transfer completed in {:?}", duration);
            }
            TransferEvent::Failed { transfer_id, error } => {
                eprintln!("Transfer failed: {}", error);
            }
        }
    }
});
```

### 1.6 Identity Management

**Creating and Loading Identities:**

```rust
use wraith_core::node::{Node, Identity};
use std::path::PathBuf;

// Create node with random identity
let node = Node::new_random(config)?;

// Create identity from existing Ed25519 keypair
let ed25519_keypair = /* load keypair */;
let identity = Identity::from_keypair(ed25519_keypair)?;
let node = Node::new_from_identity(identity, config)?;

// Load identity from file
let identity_path = PathBuf::from("~/.config/wraith/keypair.secret");
let identity = Identity::load_from_file(&identity_path)?;
let node = Node::new_from_identity(identity, config)?;

// Save identity to file (with optional passphrase)
let passphrase = Some(b"my-secure-passphrase");
identity.save_to_file(&identity_path, passphrase)?;

// Get node ID (derived from Ed25519 public key)
let node_id = node.id();
println!("Node ID: {}", node_id);
```

### 1.7 Multi-Peer Downloads

**Enable Multi-Peer Downloads:**

```rust
use wraith_core::node::{Node, MultiPeerStrategy};

// Configure multi-peer strategy
let mut config = NodeConfig::default();
config.transfer.multi_peer_enabled = true;
config.transfer.max_peers_per_download = 5;
config.transfer.multi_peer_strategy = MultiPeerStrategy::Adaptive;

let node = Node::new_random(config)?;

// Multi-peer downloads happen automatically
// When receiving a file, WRAITH will:
// 1. Discover all peers with the file via DHT
// 2. Establish sessions with multiple peers
// 3. Assign chunks to peers based on strategy
// 4. Download chunks in parallel
// 5. Reassemble and verify

// Monitor multi-peer download
let transfer_id = /* transfer ID */;
loop {
    let info = node.get_transfer_info(transfer_id).await?;

    println!("Downloading from {} peers:", info.peer_count);
    for peer_info in info.peers {
        println!("  {}: {:.1} MB/s ({} chunks)",
                 peer_info.peer_id,
                 peer_info.throughput_mbps,
                 peer_info.chunks_assigned);
    }

    if info.completed {
        break;
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

---

## 2. Protocol Integration

### 2.1 Wire Format Integration

**Frame Structure:**

WRAITH uses a layered frame format:

```
Outer Packet (Wire Format):
┌────────────────────────────────────┐
│  Connection ID (8 bytes)           │
├────────────────────────────────────┤
│  Encrypted Payload (variable)      │
├────────────────────────────────────┤
│  Authentication Tag (16 bytes)     │
└────────────────────────────────────┘

Inner Frame (After Decryption):
┌────────────────────────────────────┐
│  Nonce (8 bytes)                   │
├────────────────────────────────────┤
│  Frame Type (1 byte)               │
│  Flags (1 byte)                    │
│  Stream ID (2 bytes)               │
│  Sequence Number (4 bytes)         │
│  File Offset (8 bytes)             │
│  Payload Length (2 bytes)          │
│  Reserved (2 bytes)                │
├────────────────────────────────────┤
│  Payload Data (variable)           │
├────────────────────────────────────┤
│  Padding (variable)                │
└────────────────────────────────────┘
```

**Frame Types:**

```rust
use wraith_core::frame::FrameType;

pub enum FrameType {
    Data = 0x01,        // File data payload
    Ack = 0x02,         // Selective acknowledgment
    Control = 0x03,     // Stream management
    Rekey = 0x04,       // Forward secrecy ratchet
    Ping = 0x05,        // Keepalive / RTT measurement
    Pong = 0x06,        // Response to PING
    Close = 0x07,       // Session termination
    Pad = 0x08,         // Cover traffic (no payload)
    StreamOpen = 0x09,  // New stream initiation
    StreamClose = 0x0A, // Stream termination
    StreamReset = 0x0B, // Abort stream with error
    PathChallenge = 0x0C, // Connection migration challenge
    PathResponse = 0x0D,  // Connection migration response
    Resume = 0x0E,      // Resume interrupted transfer
    ChunkRequest = 0x0F, // Request specific chunks
}
```

### 2.2 Cryptographic Protocol

**Noise_XX Handshake:**

WRAITH uses the Noise_XX pattern for mutual authentication:

```
Initiator                  Responder
--------                   ---------
-> e                       (ephemeral key)
                      <- e, ee, s, es
-> s, se                   (static keys exchanged)
```

**Handshake Implementation:**

```rust
use wraith_crypto::noise::{NoiseHandshake, HandshakeRole};
use wraith_crypto::keys::{Ed25519Keypair, X25519Keypair};

// Initiator side
let ed25519_keypair = Ed25519Keypair::generate();
let x25519_keypair = X25519Keypair::from_ed25519(&ed25519_keypair);

let mut handshake = NoiseHandshake::new(
    HandshakeRole::Initiator,
    x25519_keypair,
)?;

// Send message 1: e
let msg1 = handshake.write_message(&[])?;
send_to_peer(&msg1).await?;

// Receive message 2: e, ee, s, es
let msg2 = receive_from_peer().await?;
handshake.read_message(&msg2)?;

// Send message 3: s, se
let msg3 = handshake.write_message(&[])?;
send_to_peer(&msg3).await?;

// Handshake complete, get transport keys
let session_crypto = handshake.into_transport_mode()?;
```

**AEAD Encryption:**

```rust
use wraith_crypto::aead::{AeadCipher, Nonce};

// Encrypt frame payload
let nonce = Nonce::new(session_salt, packet_counter);
let plaintext = frame.encode()?;
let ciphertext = session_crypto.encrypt(&nonce, &plaintext, &connection_id)?;

// Decrypt frame payload
let plaintext = session_crypto.decrypt(&nonce, &ciphertext, &connection_id)?;
let frame = Frame::decode(&plaintext)?;
```

**Key Ratcheting:**

```rust
use wraith_crypto::ratchet::DoubleRatchet;

// Initialize ratchet from handshake
let mut ratchet = DoubleRatchet::from_handshake(
    handshake_keys.root_key,
    handshake_keys.chain_key,
)?;

// Ratchet on every frame (symmetric ratchet)
let (send_key, recv_key) = ratchet.ratchet_symmetric()?;

// Periodic DH ratchet (every 2 minutes or 1M packets)
if should_ratchet_dh() {
    let new_ephemeral = X25519Keypair::generate();
    ratchet.ratchet_dh(new_ephemeral, peer_ephemeral_public)?;
}
```

### 2.3 Session State Machine

**Session States:**

```rust
pub enum SessionState {
    Idle,           // No handshake initiated
    Handshaking,    // Noise_XX handshake in progress
    Established,    // Session active
    Migrating,      // Connection migration in progress
    Closing,        // Graceful shutdown initiated
    Closed,         // Session terminated
    Failed,         // Session failed (error state)
}
```

**State Transitions:**

```rust
use wraith_core::session::{Session, SessionEvent};

let mut session = Session::new(peer_id, connection_id);

// State machine event loop
loop {
    match session.state() {
        SessionState::Idle => {
            // Initiate handshake
            session.start_handshake().await?;
        }
        SessionState::Handshaking => {
            // Process handshake messages
            let event = session.poll_event().await?;
            if let SessionEvent::HandshakeComplete = event {
                println!("Session established!");
            }
        }
        SessionState::Established => {
            // Process data frames
            let frame = session.receive_frame().await?;
            handle_frame(frame).await?;
        }
        SessionState::Closing => {
            // Wait for graceful shutdown
            session.wait_close().await?;
        }
        SessionState::Closed | SessionState::Failed => {
            break;
        }
        _ => {}
    }
}
```

### 2.4 Stream Multiplexing

**Creating Streams:**

```rust
use wraith_core::stream::{Stream, StreamId};

// Open new stream for file transfer
let stream_id = session.open_stream().await?;
let mut stream = session.get_stream(stream_id)?;

// Send data on stream
let data = b"file chunk data...";
stream.send(data).await?;

// Receive data from stream
let received = stream.receive().await?;

// Close stream gracefully
stream.close().await?;

// Or reset stream with error
stream.reset(StreamError::Canceled).await?;
```

**Stream Priorities:**

```rust
use wraith_core::stream::StreamPriority;

// Set stream priority for prioritized data
stream.set_priority(StreamPriority::High)?;

// Control frames always have highest priority
// Data frames follow priority order: Urgent > High > Normal > Low
```

---

## 3. Transport Integration

### 3.1 Custom Transport Implementation

**Transport Trait:**

```rust
use wraith_transport::{AsyncTransport, Packet};
use async_trait::async_trait;

#[async_trait]
pub trait AsyncTransport: Send + Sync {
    /// Send packet to destination
    async fn send(&self, packet: &Packet, dest: SocketAddr) -> Result<(), TransportError>;

    /// Receive packet (non-blocking)
    async fn recv(&self) -> Result<(Packet, SocketAddr), TransportError>;

    /// Get maximum transmission unit
    fn mtu(&self) -> usize;

    /// Get local bind address
    fn local_addr(&self) -> SocketAddr;
}
```

**Example: UDP Transport:**

```rust
use wraith_transport::{AsyncTransport, Packet, TransportError};
use tokio::net::UdpSocket;

pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    mtu: usize,
}

impl UdpTransport {
    pub async fn bind(addr: SocketAddr) -> Result<Self, TransportError> {
        let socket = UdpSocket::bind(addr).await?;

        // Set socket options
        socket.set_broadcast(false)?;

        // Increase buffer sizes
        let send_buf_size = 2 * 1024 * 1024; // 2 MB
        let recv_buf_size = 2 * 1024 * 1024;
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = socket.as_raw_fd();
            unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_SNDBUF,
                    &send_buf_size as *const _ as *const libc::c_void,
                    std::mem::size_of_val(&send_buf_size) as u32,
                );
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_RCVBUF,
                    &recv_buf_size as *const _ as *const libc::c_void,
                    std::mem::size_of_val(&recv_buf_size) as u32,
                );
            }
        }

        Ok(Self {
            socket: Arc::new(socket),
            mtu: 1472, // Standard MTU minus headers
        })
    }
}

#[async_trait]
impl AsyncTransport for UdpTransport {
    async fn send(&self, packet: &Packet, dest: SocketAddr) -> Result<(), TransportError> {
        let bytes = packet.as_bytes();
        self.socket.send_to(bytes, dest).await?;
        Ok(())
    }

    async fn recv(&self) -> Result<(Packet, SocketAddr), TransportError> {
        let mut buf = vec![0u8; self.mtu];
        let (len, src) = self.socket.recv_from(&mut buf).await?;
        buf.truncate(len);

        let packet = Packet::from_bytes(&buf)?;
        Ok((packet, src))
    }

    fn mtu(&self) -> usize {
        self.mtu
    }

    fn local_addr(&self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }
}
```

### 3.2 AF_XDP Integration (Linux)

**AF_XDP Transport:**

```rust
use wraith_transport::afxdp::{AfXdpTransport, UmemConfig};

// Create UMEM (shared memory for zero-copy)
let umem_config = UmemConfig {
    frame_count: 4096,
    frame_size: 2048,
    fill_size: 2048,
    completion_size: 2048,
};

// Bind AF_XDP socket
let transport = AfXdpTransport::bind(
    "eth0",       // interface
    0,            // queue ID
    umem_config,
).await?;

// Use transport with batch operations
let packets = vec![packet1, packet2, packet3];
transport.send_batch(&packets, dest).await?;

let received = transport.recv_batch(32).await?; // Receive up to 32 packets
```

**Requirements:**
- Linux kernel 6.2+
- NIC with AF_XDP support
- CAP_NET_RAW capability or root
- XDP program loaded on interface

### 3.3 io_uring File I/O

**io_uring Integration:**

```rust
use wraith_files::io_uring::{IoUringBackend, IoUringConfig};

// Create io_uring backend
let config = IoUringConfig {
    ring_size: 2048,
    sqpoll_enabled: true,
    iopoll_enabled: true,
};

let backend = IoUringBackend::new(config)?;

// Read file asynchronously
let file_path = PathBuf::from("large_file.dat");
let offset = 0;
let length = 1024 * 1024; // 1 MB
let buffer = backend.read_at(&file_path, offset, length).await?;

// Write file asynchronously
let data = vec![0u8; 1024 * 1024];
backend.write_at(&file_path, offset, &data).await?;

// Batch I/O operations
let ops = vec![
    IoOp::Read { file_id: 0, offset: 0, length: 1024 },
    IoOp::Write { file_id: 1, offset: 0, data: vec![...] },
    IoOp::Sync { file_id: 0 },
];

let results = backend.submit_batch(ops).await?;
```

---

## 4. Discovery Integration

### 4.1 DHT Integration

**Kademlia DHT:**

```rust
use wraith_discovery::dht::{Kademlia, NodeId, Config};

// Create DHT node
let node_id = NodeId::from_public_key(&ed25519_public_key);
let config = Config {
    k: 20,              // Replication factor
    alpha: 3,           // Lookup concurrency
    refresh_interval: Duration::from_secs(3600), // 1 hour
};

let mut dht = Kademlia::new(node_id, config);

// Bootstrap from known nodes
let bootstrap_nodes = vec![
    ("bootstrap1.wraith.network:41641".parse()?, bootstrap_node_id1),
    ("bootstrap2.wraith.network:41641".parse()?, bootstrap_node_id2),
];

for (addr, node_id) in bootstrap_nodes {
    dht.add_node(node_id, addr);
}

dht.bootstrap().await?;

// Announce yourself in DHT
let info_hash = compute_info_hash(&file_hash, &group_secret);
dht.announce(info_hash, your_addr).await?;

// Find peers sharing a file
let peers = dht.lookup_peers(info_hash).await?;
for (peer_id, peer_addr) in peers {
    println!("Found peer: {} at {}", peer_id, peer_addr);
}
```

**Privacy-Enhanced DHT:**

```rust
use wraith_discovery::dht::PrivacyDht;
use blake3::Hasher;

// Compute keyed info_hash (prevents real hash exposure)
fn compute_info_hash(file_hash: &[u8; 32], group_secret: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new_keyed(group_secret);
    hasher.update(file_hash);
    let hash = hasher.finalize();
    *hash.as_bytes()
}

// Only peers with the group_secret can:
// 1. Derive the same info_hash
// 2. Find peers in DHT
// 3. Verify file authenticity

// This provides privacy-preserving peer discovery
```

### 4.2 NAT Traversal

**STUN Client:**

```rust
use wraith_discovery::stun::{StunClient, NatType};

// Create STUN client
let stun_server = "stun.wraith.network:41641".parse()?;
let stun_client = StunClient::new(stun_server);

// Detect NAT type
let nat_type = stun_client.detect_nat_type().await?;
println!("NAT Type: {:?}", nat_type);

// Get public address
let public_addr = stun_client.get_public_addr().await?;
println!("Public Address: {}", public_addr);

match nat_type {
    NatType::FullCone => {
        println!("Direct connections should work");
    }
    NatType::Symmetric => {
        println!("Need relay fallback");
    }
    _ => {
        println!("UDP hole punching may work");
    }
}
```

**UDP Hole Punching:**

```rust
use wraith_discovery::nat::{HolePuncher, IceCandidate};

// Gather ICE candidates
let hole_puncher = HolePuncher::new(local_socket);
let candidates = hole_puncher.gather_candidates().await?;

// Exchange candidates with peer (out-of-band via DHT/relay)
send_candidates_to_peer(&candidates).await?;
let peer_candidates = receive_candidates_from_peer().await?;

// Attempt hole punching
let connection = hole_puncher.punch_hole(&peer_candidates).await?;

if connection.is_some() {
    println!("Hole punching successful!");
} else {
    println!("Hole punching failed, using relay");
}
```

**Relay Fallback:**

```rust
use wraith_discovery::relay::{RelayClient, RelayServer};

// Connect to relay server
let relay_addr = "relay.wraith.network:41641".parse()?;
let relay_client = RelayClient::connect(relay_addr).await?;

// Forward packets through relay
relay_client.forward_to_peer(peer_id, packet).await?;

// Receive packets from relay
let (packet, from_peer) = relay_client.receive().await?;
```

---

## 5. Error Handling

### 5.1 Error Types

**WRAITH Error Hierarchy:**

```rust
use wraith_core::node::NodeError;

pub enum NodeError {
    // Transport errors
    TransportInit(String),
    Transport(TransportError),

    // Crypto errors
    Crypto(CryptoError),
    SessionEstablishment(String),
    SessionNotFound(NodeId),

    // Transfer errors
    Transfer(TransferError),
    TransferNotFound(TransferId),

    // I/O errors
    Io(std::io::Error),

    // Discovery errors
    Discovery(DiscoveryError),
    NatTraversal(String),

    // Migration errors
    Migration(String),

    // Configuration errors
    InvalidConfig(String),

    // Timeout errors
    Timeout(String),

    // Peer errors
    PeerNotFound(NodeId),
    Handshake(String),

    // State errors
    InvalidState(String),

    // Channel errors
    Channel(String),

    // Generic errors
    Other(String),
}
```

### 5.2 Error Handling Patterns

**Result Types:**

```rust
use wraith_core::node::{Node, NodeError};

// All WRAITH APIs return Result types
type Result<T> = std::result::Result<T, NodeError>;

// Handle errors with pattern matching
match node.send_file(peer_id, file_path).await {
    Ok(transfer_id) => {
        println!("Transfer started: {}", transfer_id);
    }
    Err(NodeError::SessionNotFound(peer_id)) => {
        // Establish session first
        node.establish_session(peer_id).await?;
        // Retry
        node.send_file(peer_id, file_path).await?;
    }
    Err(NodeError::PeerNotFound(peer_id)) => {
        // Discover peer via DHT
        let peer_addr = node.lookup_peer(peer_id).await?;
        // Retry
        node.send_file(peer_id, file_path).await?;
    }
    Err(e) => {
        eprintln!("Transfer failed: {}", e);
        return Err(e.into());
    }
}
```

### 5.3 Retry Logic

**Exponential Backoff:**

```rust
use wraith_core::node::CircuitBreaker;

// Circuit breaker prevents cascading failures
let circuit_breaker = CircuitBreaker::new(
    5,                           // failure_threshold
    Duration::from_secs(30),     // timeout
    Duration::from_secs(5),      // recovery_time
);

// Retry with exponential backoff
let mut backoff = Duration::from_millis(100);
let max_retries = 5;

for attempt in 0..max_retries {
    match circuit_breaker.call(|| async {
        node.establish_session(peer_id).await
    }).await {
        Ok(session_id) => {
            println!("Session established: {}", session_id);
            break;
        }
        Err(e) if attempt < max_retries - 1 => {
            eprintln!("Attempt {} failed: {}. Retrying in {:?}...",
                     attempt + 1, e, backoff);
            tokio::time::sleep(backoff).await;
            backoff *= 2; // Exponential backoff
        }
        Err(e) => {
            eprintln!("All retries failed: {}", e);
            return Err(e.into());
        }
    }
}
```

### 5.4 Recovery Strategies

**Connection Loss Recovery:**

```rust
// Automatic reconnection with resume
async fn transfer_with_resume(
    node: &Node,
    peer_id: NodeId,
    file_path: PathBuf,
) -> Result<()> {
    let transfer_id = node.send_file(peer_id, file_path.clone()).await?;

    loop {
        match node.wait_for_transfer(transfer_id).await {
            Ok(()) => {
                println!("Transfer complete!");
                return Ok(());
            }
            Err(NodeError::SessionNotFound(_)) => {
                // Connection lost, resume transfer
                println!("Connection lost, resuming...");
                node.establish_session(peer_id).await?;
                node.resume_transfer(transfer_id).await?;
            }
            Err(e) => {
                eprintln!("Transfer failed: {}", e);
                return Err(e.into());
            }
        }
    }
}
```

**Rate Limiting:**

```rust
use wraith_core::node::RateLimiter;

// Rate limiter prevents DoS
let rate_limiter = RateLimiter::new(
    100,                      // max_connections_per_ip
    Duration::from_secs(60), // window
);

// Check rate limit before accepting connection
if !rate_limiter.check_and_update(peer_addr.ip()) {
    println!("Rate limit exceeded for {}", peer_addr.ip());
    return Err(NodeError::RateLimited);
}
```

---

## Conclusion

This integration guide covered the essential APIs and patterns for integrating WRAITH Protocol into your applications. For more detailed information, consult the following resources:

**Additional Documentation:**
- [API Reference](engineering/api-reference.md) - Complete API documentation
- [Protocol Technical Details](../ref-docs/protocol_technical_details.md) - Wire format specification
- [Security Model](architecture/security-model.md) - Cryptographic details
- [Performance Benchmarks](testing/performance-benchmarks.md) - Performance characteristics

**Example Code:**
- [Integration Tests](../tests/integration_tests.rs) - Complete integration examples
- [Benchmarks](../benches) - Performance testing examples

**Community Support:**
- [GitHub Discussions](https://github.com/doublegate/WRAITH-Protocol/discussions)
- [Issue Tracker](https://github.com/doublegate/WRAITH-Protocol/issues)

---

**WRAITH Protocol** - Secure, Fast, Invisible File Transfer

**Version:** 1.0.0 | **License:** MIT | **Language:** Rust 2024
