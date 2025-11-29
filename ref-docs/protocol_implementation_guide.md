# Protocol Implementation Guide

## Decentralized Secure File Transfer Protocol — Rust Implementation Reference

**Document Version:** 1.0.0-DRAFT  
**Status:** Implementation Guide  
**Target Runtime:** Linux 6.2+ (kernel features), Rust 2021 Edition  
**Architecture:** x86_64 (primary), aarch64 (secondary)  

---

## Table of Contents

1. [Implementation Architecture](#1-implementation-architecture)
2. [Project Structure](#2-project-structure)
3. [Core Module Specifications](#3-core-module-specifications)
4. [Kernel Acceleration Layer](#4-kernel-acceleration-layer)
5. [Cryptographic Implementation](#5-cryptographic-implementation)
6. [Transport Layer Implementation](#6-transport-layer-implementation)
7. [File Transfer Engine](#7-file-transfer-engine)
8. [Discovery and Relay Integration](#8-discovery-and-relay-integration)
9. [API Design](#9-api-design)
10. [Testing Strategy](#10-testing-strategy)
11. [Performance Tuning](#11-performance-tuning)
12. [Build and Deployment](#12-build-and-deployment)
13. [Security Hardening](#13-security-hardening)

---

## 1. Implementation Architecture

### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CLI Interface                                  │
│                    (clap, config parsing, progress display)                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                              Public API                                     │
│              (Node, Session, Transfer, PeerDiscovery traits)               │
├───────────────────────────────┬─────────────────────────────────────────────┤
│       File Transfer Engine    │          Discovery Manager                  │
│  (chunking, integrity, resume)│    (DHT, relay, NAT traversal)             │
├───────────────────────────────┴─────────────────────────────────────────────┤
│                           Session Manager                                   │
│        (multiplexing, streams, flow control, congestion control)           │
├─────────────────────────────────────────────────────────────────────────────┤
│                         Cryptographic Transport                             │
│           (Noise_XX, AEAD, key ratcheting, Elligator2)                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                         Obfuscation Layer                                   │
│              (padding, timing, cover traffic, mimicry)                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                      Kernel Acceleration Layer                              │
│              (AF_XDP, XDP/eBPF, io_uring, zero-copy)                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                           Linux Kernel 6.x                                  │
│                    (UDP sockets, NIC driver, DMA)                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Threading Model

The implementation uses a **thread-per-core** model for maximum performance:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Main Thread                                       │
│           (initialization, signal handling, graceful shutdown)             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │  IO Worker  │  │  IO Worker  │  │  IO Worker  │  │  IO Worker  │  ...   │
│  │   Core 0    │  │   Core 1    │  │   Core 2    │  │   Core 3    │        │
│  │             │  │             │  │             │  │             │        │
│  │ AF_XDP Sock │  │ AF_XDP Sock │  │ AF_XDP Sock │  │ AF_XDP Sock │        │
│  │ io_uring    │  │ io_uring    │  │ io_uring    │  │ io_uring    │        │
│  │ Sessions[]  │  │ Sessions[]  │  │ Sessions[]  │  │ Sessions[]  │        │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘        │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────┐  ┌─────────────────────────────────────────┐  │
│  │     DHT Worker          │  │           Relay Worker                  │  │
│  │  (background discovery) │  │   (relay connections, signaling)        │  │
│  └─────────────────────────┘  └─────────────────────────────────────────┘  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        Timer Wheel                                   │   │
│  │            (retransmissions, keepalives, rekeying)                  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key Principles:**

- **No locks in hot path:** Sessions are pinned to cores; no cross-thread sharing
- **Work stealing only for imbalance:** Optional work stealing when cores are idle
- **NUMA awareness:** Memory allocation respects NUMA topology
- **CPU pinning:** Workers pinned via `pthread_setaffinity_np`

### 1.3 Memory Architecture

```rust
/// Per-core memory allocation strategy
pub struct CoreMemory {
    /// UMEM for AF_XDP (must be page-aligned, NUMA-local)
    umem: UmemRegion,
    
    /// Packet buffers (pre-allocated pool)
    packet_pool: PacketPool,
    
    /// Session state (per-connection)
    sessions: Slab<Session>,
    
    /// Scratch space for crypto operations
    crypto_scratch: CryptoScratch,
}

impl CoreMemory {
    pub fn allocate_on_numa_node(node: usize, config: &Config) -> Result<Self> {
        // Use libnuma for NUMA-local allocation
        let umem = UmemRegion::allocate_numa(
            config.umem_size,
            config.frame_size,
            node,
        )?;
        
        let packet_pool = PacketPool::new_numa(
            config.packet_pool_size,
            node,
        )?;
        
        Ok(Self {
            umem,
            packet_pool,
            sessions: Slab::with_capacity(config.max_sessions_per_core),
            crypto_scratch: CryptoScratch::new(),
        })
    }
}
```

---

## 2. Project Structure

### 2.1 Workspace Layout

```
protocol/
├── Cargo.toml                    # Workspace manifest
├── README.md                     # Project overview
├── LICENSE                       # License file
│
├── crates/
│   ├── protocol-core/           # Core protocol implementation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── frame.rs          # Frame encoding/decoding
│   │       ├── session.rs        # Session state machine
│   │       ├── stream.rs         # Stream multiplexing
│   │       ├── congestion.rs     # BBR congestion control
│   │       └── error.rs          # Error types
│   │
│   ├── protocol-crypto/         # Cryptographic primitives
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── noise.rs          # Noise_XX handshake
│   │       ├── aead.rs           # XChaCha20-Poly1305
│   │       ├── elligator.rs      # Elligator2 encoding
│   │       ├── ratchet.rs        # Key ratcheting
│   │       └── random.rs         # CSPRNG wrapper
│   │
│   ├── protocol-transport/      # Network transport layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── xdp.rs            # XDP program loader
│   │       ├── af_xdp.rs         # AF_XDP socket management
│   │       ├── io_uring.rs       # io_uring integration
│   │       ├── udp.rs            # Fallback UDP sockets
│   │       └── worker.rs         # Per-core worker loop
│   │
│   ├── protocol-obfuscation/    # Traffic obfuscation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── padding.rs        # Packet padding
│   │       ├── timing.rs         # Timing obfuscation
│   │       ├── cover.rs          # Cover traffic generation
│   │       └── mimicry/
│   │           ├── mod.rs
│   │           ├── https.rs      # TLS record mimicry
│   │           ├── websocket.rs  # WebSocket mimicry
│   │           └── doh.rs        # DNS-over-HTTPS tunnel
│   │
│   ├── protocol-discovery/      # Peer discovery
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── dht.rs            # Privacy-enhanced DHT
│   │       ├── relay.rs          # DERP-style relay
│   │       ├── nat.rs            # NAT traversal
│   │       └── stun.rs           # Endpoint discovery
│   │
│   ├── protocol-files/          # File transfer engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── chunker.rs        # File chunking
│   │       ├── hasher.rs         # BLAKE3 tree hashing
│   │       ├── transfer.rs       # Transfer state machine
│   │       └── resume.rs         # Resume/seek support
│   │
│   ├── protocol-xdp/            # eBPF/XDP programs (Aya)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── main.rs           # XDP program entry point
│   │
│   └── protocol-cli/            # Command-line interface
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── send.rs       # Send file command
│           │   ├── receive.rs    # Receive file command
│           │   ├── serve.rs      # Daemon mode
│           │   └── config.rs     # Configuration management
│           └── ui/
│               ├── mod.rs
│               └── progress.rs   # Progress display
│
├── xtask/                       # Build automation
│   ├── Cargo.toml
│   └── src/
│       └── main.rs               # cargo xtask commands
│
├── tests/                       # Integration tests
│   ├── integration/
│   │   ├── handshake_test.rs
│   │   ├── transfer_test.rs
│   │   └── nat_test.rs
│   └── fixtures/
│       └── test_files/
│
├── benches/                     # Benchmarks
│   ├── crypto_bench.rs
│   ├── throughput_bench.rs
│   └── latency_bench.rs
│
└── docs/                        # Documentation
    ├── protocol_spec.md
    ├── implementation.md
    └── security.md
```

### 2.2 Cargo Workspace Configuration

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/protocol-core",
    "crates/protocol-crypto",
    "crates/protocol-transport",
    "crates/protocol-obfuscation",
    "crates/protocol-discovery",
    "crates/protocol-files",
    "crates/protocol-xdp",
    "crates/protocol-cli",
    "xtask",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "MIT OR Apache-2.0"
repository = "https://github.com/example/protocol"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }
glommio = "0.9"  # Thread-per-core runtime alternative

# Cryptography
chacha20poly1305 = "0.10"
x25519-dalek = { version = "2.0", features = ["static_secrets"] }
ed25519-dalek = { version = "2.0", features = ["rand_core"] }
blake3 = "1.5"
snow = "0.9"  # Noise Protocol
rand = "0.8"
zeroize = { version = "1.7", features = ["derive"] }

# Kernel interfaces
aya = "0.12"
aya-ebpf = "0.1"
libbpf-sys = "1.3"
io-uring = "0.7"
socket2 = "0.5"

# Networking
quinn = "0.10"  # QUIC fallback
rustls = "0.22"
webpki = "0.22"

# DHT
libp2p = { version = "0.53", features = ["kad", "noise", "tcp", "dns"] }

# Serialization
bincode = "1.3"
serde = { version = "1.0", features = ["derive"] }

# CLI
clap = { version = "4.4", features = ["derive"] }
indicatif = "0.17"  # Progress bars
console = "0.15"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Testing
proptest = "1.4"
criterion = "0.5"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

[profile.release]
lto = "thin"
codegen-units = 1
panic = "abort"
strip = true

[profile.bench]
inherits = "release"
debug = true
```

---

## 3. Core Module Specifications

### 3.1 Frame Module (`protocol-core/src/frame.rs`)

```rust
//! Frame encoding and decoding for the protocol wire format.
//!
//! This module implements zero-copy parsing of protocol frames with
//! careful attention to alignment for DMA efficiency. All multi-byte
//! fields are big-endian (network byte order).
//!
//! # Frame Structure
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │ Nonce (8B) │ Type (1B) │ Flags (1B) │ StreamID (2B) │     │
//! │ SeqNum (4B) │ Offset (8B) │ PayloadLen (2B) │ Reserved (2B)│
//! │ Payload (variable) │ Padding (variable)                    │
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust
//! use protocol_core::frame::{Frame, FrameType, FrameFlags};
//!
//! // Parsing a received packet (zero-copy)
//! let frame = Frame::parse(&encrypted_payload)?;
//!
//! // Building a frame for transmission
//! let frame = Frame::builder()
//!     .frame_type(FrameType::Data)
//!     .stream_id(42)
//!     .sequence(1000)
//!     .offset(0)
//!     .payload(&chunk_data)
//!     .build()?;
//! ```

use std::mem::size_of;
use thiserror::Error;
use zeroize::Zeroize;

/// Fixed header size in bytes
pub const HEADER_SIZE: usize = 28;

/// Minimum valid frame size (header only, no payload)
pub const MIN_FRAME_SIZE: usize = HEADER_SIZE;

/// Maximum payload size for standard MTU
pub const MAX_PAYLOAD_STANDARD: usize = 1428;

/// Maximum payload size for jumbo frames
pub const MAX_PAYLOAD_JUMBO: usize = 8928;

/// Frame types as defined in protocol specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameType {
    Reserved = 0x00,
    Data = 0x01,
    Ack = 0x02,
    Control = 0x03,
    Rekey = 0x04,
    Ping = 0x05,
    Pong = 0x06,
    Close = 0x07,
    Pad = 0x08,
    StreamOpen = 0x09,
    StreamClose = 0x0A,
    StreamReset = 0x0B,
    WindowUpdate = 0x0C,
    GoAway = 0x0D,
    PathChallenge = 0x0E,
    PathResponse = 0x0F,
}

impl TryFrom<u8> for FrameType {
    type Error = FrameError;
    
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Err(FrameError::ReservedFrameType),
            0x01 => Ok(Self::Data),
            0x02 => Ok(Self::Ack),
            0x03 => Ok(Self::Control),
            0x04 => Ok(Self::Rekey),
            0x05 => Ok(Self::Ping),
            0x06 => Ok(Self::Pong),
            0x07 => Ok(Self::Close),
            0x08 => Ok(Self::Pad),
            0x09 => Ok(Self::StreamOpen),
            0x0A => Ok(Self::StreamClose),
            0x0B => Ok(Self::StreamReset),
            0x0C => Ok(Self::WindowUpdate),
            0x0D => Ok(Self::GoAway),
            0x0E => Ok(Self::PathChallenge),
            0x0F => Ok(Self::PathResponse),
            0x10..=0x1F => Err(FrameError::ReservedFrameType),
            0x20..=0x3F => Ok(Self::Reserved), // Extension range
            _ => Err(FrameError::InvalidFrameType(value)),
        }
    }
}

/// Frame flags bitmap
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameFlags(u8);

impl FrameFlags {
    pub const SYN: u8 = 0b0000_0001;
    pub const FIN: u8 = 0b0000_0010;
    pub const ACK: u8 = 0b0000_0100;
    pub const PRI: u8 = 0b0000_1000;
    pub const CMP: u8 = 0b0001_0000;
    
    pub fn new() -> Self {
        Self(0)
    }
    
    pub fn with_syn(mut self) -> Self {
        self.0 |= Self::SYN;
        self
    }
    
    pub fn with_fin(mut self) -> Self {
        self.0 |= Self::FIN;
        self
    }
    
    pub fn is_syn(&self) -> bool {
        self.0 & Self::SYN != 0
    }
    
    pub fn is_fin(&self) -> bool {
        self.0 & Self::FIN != 0
    }
    
    pub fn is_compressed(&self) -> bool {
        self.0 & Self::CMP != 0
    }
}

/// Errors during frame parsing
#[derive(Debug, Error)]
pub enum FrameError {
    #[error("frame too short: expected at least {expected}, got {actual}")]
    TooShort { expected: usize, actual: usize },
    
    #[error("invalid frame type: 0x{0:02X}")]
    InvalidFrameType(u8),
    
    #[error("reserved frame type used")]
    ReservedFrameType,
    
    #[error("payload length exceeds packet size")]
    PayloadOverflow,
    
    #[error("invalid padding")]
    InvalidPadding,
}

/// Zero-copy frame view into a packet buffer
#[derive(Debug)]
pub struct Frame<'a> {
    /// Raw frame bytes (header + payload + padding)
    raw: &'a [u8],
    /// Parsed header for fast access
    header: FrameHeader,
}

/// Parsed frame header (28 bytes)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct FrameHeader {
    nonce: [u8; 8],
    frame_type: u8,
    flags: u8,
    stream_id: [u8; 2],
    sequence: [u8; 4],
    offset: [u8; 8],
    payload_len: [u8; 2],
    reserved: [u8; 2],
}

impl<'a> Frame<'a> {
    /// Parse a frame from raw bytes (zero-copy)
    ///
    /// # Arguments
    /// * `data` - Decrypted frame bytes (without outer CID/tag)
    ///
    /// # Returns
    /// * `Ok(Frame)` - Parsed frame view
    /// * `Err(FrameError)` - Parse failure
    pub fn parse(data: &'a [u8]) -> Result<Self, FrameError> {
        if data.len() < HEADER_SIZE {
            return Err(FrameError::TooShort {
                expected: HEADER_SIZE,
                actual: data.len(),
            });
        }
        
        // Safety: We've verified the slice is large enough
        let header_bytes: &[u8; HEADER_SIZE] = data[..HEADER_SIZE]
            .try_into()
            .unwrap();
        
        // Parse header fields (safe transmute via pointer cast)
        let header = unsafe {
            std::ptr::read_unaligned(header_bytes.as_ptr() as *const FrameHeader)
        };
        
        // Validate frame type
        let _ = FrameType::try_from(header.frame_type)?;
        
        // Validate payload length
        let payload_len = u16::from_be_bytes(header.payload_len) as usize;
        if HEADER_SIZE + payload_len > data.len() {
            return Err(FrameError::PayloadOverflow);
        }
        
        Ok(Self { raw: data, header })
    }
    
    /// Get the frame type
    pub fn frame_type(&self) -> FrameType {
        // Safe: validated in parse()
        FrameType::try_from(self.header.frame_type).unwrap()
    }
    
    /// Get the frame flags
    pub fn flags(&self) -> FrameFlags {
        FrameFlags(self.header.flags)
    }
    
    /// Get the stream ID
    pub fn stream_id(&self) -> u16 {
        u16::from_be_bytes(self.header.stream_id)
    }
    
    /// Get the sequence number
    pub fn sequence(&self) -> u32 {
        u32::from_be_bytes(self.header.sequence)
    }
    
    /// Get the file offset (for DATA frames)
    pub fn offset(&self) -> u64 {
        u64::from_be_bytes(self.header.offset)
    }
    
    /// Get the nonce bytes
    pub fn nonce(&self) -> &[u8; 8] {
        &self.header.nonce
    }
    
    /// Get the payload slice (zero-copy)
    pub fn payload(&self) -> &[u8] {
        let payload_len = u16::from_be_bytes(self.header.payload_len) as usize;
        &self.raw[HEADER_SIZE..HEADER_SIZE + payload_len]
    }
    
    /// Get the padding slice
    pub fn padding(&self) -> &[u8] {
        let payload_len = u16::from_be_bytes(self.header.payload_len) as usize;
        &self.raw[HEADER_SIZE + payload_len..]
    }
}

/// Builder for constructing frames
pub struct FrameBuilder {
    frame_type: FrameType,
    flags: FrameFlags,
    stream_id: u16,
    sequence: u32,
    offset: u64,
    payload: Vec<u8>,
    nonce: [u8; 8],
}

impl FrameBuilder {
    pub fn new() -> Self {
        Self {
            frame_type: FrameType::Data,
            flags: FrameFlags::new(),
            stream_id: 0,
            sequence: 0,
            offset: 0,
            payload: Vec::new(),
            nonce: [0u8; 8],
        }
    }
    
    pub fn frame_type(mut self, ft: FrameType) -> Self {
        self.frame_type = ft;
        self
    }
    
    pub fn flags(mut self, flags: FrameFlags) -> Self {
        self.flags = flags;
        self
    }
    
    pub fn stream_id(mut self, id: u16) -> Self {
        self.stream_id = id;
        self
    }
    
    pub fn sequence(mut self, seq: u32) -> Self {
        self.sequence = seq;
        self
    }
    
    pub fn offset(mut self, off: u64) -> Self {
        self.offset = off;
        self
    }
    
    pub fn payload(mut self, data: &[u8]) -> Self {
        self.payload = data.to_vec();
        self
    }
    
    pub fn nonce(mut self, n: [u8; 8]) -> Self {
        self.nonce = n;
        self
    }
    
    /// Build the frame into a byte buffer
    ///
    /// # Arguments
    /// * `padding_size` - Total frame size (payload + padding)
    ///
    /// # Returns
    /// * Serialized frame bytes ready for encryption
    pub fn build(self, total_size: usize) -> Result<Vec<u8>, FrameError> {
        let payload_len = self.payload.len();
        
        if total_size < HEADER_SIZE + payload_len {
            return Err(FrameError::PayloadOverflow);
        }
        
        let padding_len = total_size - HEADER_SIZE - payload_len;
        let mut buf = Vec::with_capacity(total_size);
        
        // Write header
        buf.extend_from_slice(&self.nonce);
        buf.push(self.frame_type as u8);
        buf.push(self.flags.0);
        buf.extend_from_slice(&self.stream_id.to_be_bytes());
        buf.extend_from_slice(&self.sequence.to_be_bytes());
        buf.extend_from_slice(&self.offset.to_be_bytes());
        buf.extend_from_slice(&(payload_len as u16).to_be_bytes());
        buf.extend_from_slice(&[0u8; 2]); // Reserved
        
        // Write payload
        buf.extend_from_slice(&self.payload);
        
        // Write random padding
        let mut padding = vec![0u8; padding_len];
        getrandom::fill(&mut padding).expect("CSPRNG failure");
        buf.extend_from_slice(&padding);
        
        Ok(buf)
    }
}

impl Default for FrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_frame_roundtrip() {
        let original = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(42)
            .sequence(1000)
            .offset(0)
            .payload(b"Hello, world!")
            .build(128)
            .unwrap();
        
        let parsed = Frame::parse(&original).unwrap();
        
        assert_eq!(parsed.frame_type(), FrameType::Data);
        assert_eq!(parsed.stream_id(), 42);
        assert_eq!(parsed.sequence(), 1000);
        assert_eq!(parsed.offset(), 0);
        assert_eq!(parsed.payload(), b"Hello, world!");
    }
    
    #[test]
    fn test_frame_too_short() {
        let short = [0u8; 10];
        assert!(matches!(
            Frame::parse(&short),
            Err(FrameError::TooShort { .. })
        ));
    }
}
```

### 3.2 Session Module (`protocol-core/src/session.rs`)

```rust
//! Session state machine and connection management.
//!
//! A Session represents an authenticated, encrypted connection between
//! two peers. Sessions multiplex multiple streams (file transfers) over
//! a single UDP "connection" (really just a (local_addr, remote_addr) tuple).
//!
//! # Session Lifecycle
//!
//! ```text
//! CLOSED → HANDSHAKING → ESTABLISHED → DRAINING → CLOSED
//!              ↑              │
//!              └── REKEYING ──┘
//! ```
//!
//! # Thread Safety
//!
//! Sessions are NOT thread-safe. They are pinned to a single I/O worker
//! core and accessed only from that core's event loop.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::congestion::BbrState;
use crate::frame::{Frame, FrameType};
use crate::stream::Stream;
use protocol_crypto::{SessionKeys, NoiseSession};

/// Session configuration parameters
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum concurrent streams per session
    pub max_streams: u16,
    
    /// Initial flow control window (bytes)
    pub initial_window: u64,
    
    /// Maximum flow control window (bytes)
    pub max_window: u64,
    
    /// Idle timeout before session close
    pub idle_timeout: Duration,
    
    /// Rekey interval for forward secrecy
    pub rekey_interval: Duration,
    
    /// Maximum packets before mandatory rekey
    pub rekey_packet_limit: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_streams: 16384,
            initial_window: 1024 * 1024,      // 1 MiB
            max_window: 16 * 1024 * 1024,     // 16 MiB
            idle_timeout: Duration::from_secs(30),
            rekey_interval: Duration::from_secs(120),
            rekey_packet_limit: 1_000_000,
        }
    }
}

/// Session state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Initial state, no connection
    Closed,
    
    /// Handshake in progress
    Handshaking(HandshakePhase),
    
    /// Connection established, normal operation
    Established,
    
    /// Rekeying in progress (forward secrecy)
    Rekeying,
    
    /// Graceful shutdown, draining pending data
    Draining,
    
    /// Connection migration, validating new path
    Migrating,
}

/// Handshake sub-states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakePhase {
    /// Initiator: sent Phase 1, awaiting Phase 2
    InitSent,
    
    /// Responder: received Phase 1, sent Phase 2, awaiting Phase 3
    RespSent,
    
    /// Initiator: received Phase 2, sent Phase 3
    InitComplete,
}

/// Connection identifier (derived from handshake)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId([u8; 8]);

impl ConnectionId {
    /// Special CID for handshake initiation
    pub const HANDSHAKE: Self = Self([0xFF; 8]);
    
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }
    
    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }
    
    /// Rotate low bits based on sequence number (anti-tracking)
    pub fn rotate(&self, seq: u32) -> Self {
        let mut rotated = self.0;
        let seq_bytes = seq.to_be_bytes();
        for i in 4..8 {
            rotated[i] ^= seq_bytes[i - 4];
        }
        Self(rotated)
    }
}

/// A single session with a remote peer
pub struct Session {
    /// Current state
    state: SessionState,
    
    /// Local endpoint
    local_addr: SocketAddr,
    
    /// Remote endpoint (may change during migration)
    remote_addr: SocketAddr,
    
    /// Connection identifier
    connection_id: ConnectionId,
    
    /// Cryptographic keys and state
    keys: Option<SessionKeys>,
    
    /// Noise handshake state (during HANDSHAKING)
    handshake: Option<NoiseSession>,
    
    /// Active streams (stream_id → Stream)
    streams: BTreeMap<u16, Stream>,
    
    /// Next stream ID to allocate (client: odd, server: even)
    next_stream_id: u16,
    
    /// Congestion control state
    congestion: BbrState,
    
    /// Unacknowledged packets (seq → SentPacket)
    unacked: BTreeMap<u32, SentPacket>,
    
    /// Next sequence number to send
    next_seq: u32,
    
    /// Largest acknowledged sequence number
    largest_acked: u32,
    
    /// Packets sent since last rekey
    packets_since_rekey: u64,
    
    /// Time of last rekey
    last_rekey: Instant,
    
    /// Time of last activity (for idle timeout)
    last_activity: Instant,
    
    /// Session configuration
    config: SessionConfig,
    
    /// Pending outgoing frames
    send_queue: Vec<PendingFrame>,
}

/// Metadata for sent packets (for loss detection and RTT estimation)
struct SentPacket {
    sent_time: Instant,
    size: usize,
    frame_type: FrameType,
    stream_id: Option<u16>,
    retransmittable: bool,
}

/// Frame queued for transmission
struct PendingFrame {
    frame: Vec<u8>,
    priority: u8,
    stream_id: Option<u16>,
}

impl Session {
    /// Create a new session as the connection initiator
    pub fn new_initiator(
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
        remote_static_key: &[u8; 32],
        config: SessionConfig,
    ) -> Result<Self, SessionError> {
        let handshake = NoiseSession::new_initiator(remote_static_key)?;
        
        Ok(Self {
            state: SessionState::Closed,
            local_addr,
            remote_addr,
            connection_id: ConnectionId::HANDSHAKE,
            keys: None,
            handshake: Some(handshake),
            streams: BTreeMap::new(),
            next_stream_id: 1, // Initiator uses odd IDs
            congestion: BbrState::new(),
            unacked: BTreeMap::new(),
            next_seq: 0,
            largest_acked: 0,
            packets_since_rekey: 0,
            last_rekey: Instant::now(),
            last_activity: Instant::now(),
            config,
            send_queue: Vec::new(),
        })
    }
    
    /// Create a new session as the connection responder
    pub fn new_responder(
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
        local_static_key: &[u8; 32],
        config: SessionConfig,
    ) -> Result<Self, SessionError> {
        let handshake = NoiseSession::new_responder(local_static_key)?;
        
        Ok(Self {
            state: SessionState::Closed,
            local_addr,
            remote_addr,
            connection_id: ConnectionId::HANDSHAKE,
            keys: None,
            handshake: Some(handshake),
            streams: BTreeMap::new(),
            next_stream_id: 2, // Responder uses even IDs
            congestion: BbrState::new(),
            unacked: BTreeMap::new(),
            next_seq: 0,
            largest_acked: 0,
            packets_since_rekey: 0,
            last_rekey: Instant::now(),
            last_activity: Instant::now(),
            config,
            send_queue: Vec::new(),
        })
    }
    
    /// Initiate the handshake (for initiator)
    pub fn start_handshake(&mut self) -> Result<Vec<u8>, SessionError> {
        if self.state != SessionState::Closed {
            return Err(SessionError::InvalidState);
        }
        
        let handshake = self.handshake.as_mut()
            .ok_or(SessionError::NoHandshake)?;
        
        let message = handshake.write_message_1()?;
        
        self.state = SessionState::Handshaking(HandshakePhase::InitSent);
        self.last_activity = Instant::now();
        
        Ok(message)
    }
    
    /// Process a received handshake message
    pub fn process_handshake(&mut self, message: &[u8]) -> Result<Option<Vec<u8>>, SessionError> {
        let handshake = self.handshake.as_mut()
            .ok_or(SessionError::NoHandshake)?;
        
        match self.state {
            SessionState::Closed => {
                // Responder receiving Phase 1
                handshake.read_message_1(message)?;
                let response = handshake.write_message_2()?;
                
                self.state = SessionState::Handshaking(HandshakePhase::RespSent);
                self.last_activity = Instant::now();
                
                Ok(Some(response))
            }
            
            SessionState::Handshaking(HandshakePhase::InitSent) => {
                // Initiator receiving Phase 2
                handshake.read_message_2(message)?;
                let response = handshake.write_message_3()?;
                
                // Handshake complete for initiator
                self.finalize_handshake()?;
                
                Ok(Some(response))
            }
            
            SessionState::Handshaking(HandshakePhase::RespSent) => {
                // Responder receiving Phase 3
                handshake.read_message_3(message)?;
                
                // Handshake complete for responder
                self.finalize_handshake()?;
                
                Ok(None)
            }
            
            _ => Err(SessionError::InvalidState),
        }
    }
    
    /// Complete handshake and transition to ESTABLISHED
    fn finalize_handshake(&mut self) -> Result<(), SessionError> {
        let handshake = self.handshake.take()
            .ok_or(SessionError::NoHandshake)?;
        
        // Extract session keys
        let keys = handshake.into_keys()?;
        
        // Derive connection ID
        let cid_bytes = keys.derive_connection_id();
        self.connection_id = ConnectionId::from_bytes(cid_bytes);
        
        self.keys = Some(keys);
        self.state = SessionState::Established;
        self.last_activity = Instant::now();
        self.last_rekey = Instant::now();
        
        Ok(())
    }
    
    /// Open a new stream for file transfer
    pub fn open_stream(&mut self) -> Result<u16, SessionError> {
        if self.state != SessionState::Established {
            return Err(SessionError::InvalidState);
        }
        
        if self.streams.len() >= self.config.max_streams as usize {
            return Err(SessionError::TooManyStreams);
        }
        
        let stream_id = self.next_stream_id;
        self.next_stream_id += 2; // Skip by 2 to maintain odd/even
        
        let stream = Stream::new(stream_id, self.config.initial_window);
        self.streams.insert(stream_id, stream);
        
        Ok(stream_id)
    }
    
    /// Process a received frame
    pub fn process_frame(&mut self, frame: Frame<'_>) -> Result<(), SessionError> {
        self.last_activity = Instant::now();
        
        match frame.frame_type() {
            FrameType::Data => self.handle_data(frame),
            FrameType::Ack => self.handle_ack(frame),
            FrameType::Rekey => self.handle_rekey(frame),
            FrameType::Ping => self.handle_ping(frame),
            FrameType::Close => self.handle_close(frame),
            FrameType::Pad => Ok(()), // Silently discard cover traffic
            _ => self.handle_control(frame),
        }
    }
    
    /// Check if rekeying is needed
    pub fn needs_rekey(&self) -> bool {
        if self.state != SessionState::Established {
            return false;
        }
        
        self.packets_since_rekey >= self.config.rekey_packet_limit
            || self.last_rekey.elapsed() >= self.config.rekey_interval
    }
    
    /// Initiate rekeying for forward secrecy
    pub fn initiate_rekey(&mut self) -> Result<Vec<u8>, SessionError> {
        if self.state != SessionState::Established {
            return Err(SessionError::InvalidState);
        }
        
        let keys = self.keys.as_mut()
            .ok_or(SessionError::NoKeys)?;
        
        let rekey_message = keys.create_rekey_message()?;
        
        self.state = SessionState::Rekeying;
        
        Ok(rekey_message)
    }
    
    /// Get pending frames to send
    pub fn drain_send_queue(&mut self) -> Vec<Vec<u8>> {
        let mut frames = Vec::new();
        
        // Sort by priority
        self.send_queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // Drain up to congestion window
        while let Some(pending) = self.send_queue.pop() {
            if self.congestion.can_send(pending.frame.len()) {
                frames.push(pending.frame);
            } else {
                // Put back and stop
                self.send_queue.push(pending);
                break;
            }
        }
        
        frames
    }
    
    // ... frame handlers ...
    
    fn handle_data(&mut self, frame: Frame<'_>) -> Result<(), SessionError> {
        let stream_id = frame.stream_id();
        let stream = self.streams.get_mut(&stream_id)
            .ok_or(SessionError::UnknownStream(stream_id))?;
        
        stream.receive_data(frame.sequence(), frame.offset(), frame.payload())?;
        
        Ok(())
    }
    
    fn handle_ack(&mut self, frame: Frame<'_>) -> Result<(), SessionError> {
        // Parse ACK ranges and update congestion control
        // ... implementation ...
        Ok(())
    }
    
    fn handle_rekey(&mut self, frame: Frame<'_>) -> Result<(), SessionError> {
        // Process rekeying
        // ... implementation ...
        Ok(())
    }
    
    fn handle_ping(&mut self, frame: Frame<'_>) -> Result<(), SessionError> {
        // Respond with PONG
        // ... implementation ...
        Ok(())
    }
    
    fn handle_close(&mut self, frame: Frame<'_>) -> Result<(), SessionError> {
        self.state = SessionState::Draining;
        Ok(())
    }
    
    fn handle_control(&mut self, frame: Frame<'_>) -> Result<(), SessionError> {
        // Handle other control frames
        // ... implementation ...
        Ok(())
    }
}

/// Session errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("invalid state for operation")]
    InvalidState,
    
    #[error("no handshake in progress")]
    NoHandshake,
    
    #[error("no session keys available")]
    NoKeys,
    
    #[error("too many concurrent streams")]
    TooManyStreams,
    
    #[error("unknown stream: {0}")]
    UnknownStream(u16),
    
    #[error("crypto error: {0}")]
    Crypto(#[from] protocol_crypto::CryptoError),
}
```

---

## 4. Kernel Acceleration Layer

### 4.1 XDP Program (`protocol-xdp/src/main.rs`)

```rust
//! XDP (eXpress Data Path) program for kernel-level packet filtering.
//!
//! This eBPF program runs at the NIC driver level, before the kernel
//! network stack. It filters protocol packets by connection ID and
//! redirects them to AF_XDP sockets for zero-copy userspace delivery.
//!
//! # Performance
//!
//! - Drop rate: ~26 million packets/second per core
//! - Redirect rate: ~24 million packets/second per core
//! - Latency: <1μs from NIC to userspace
//!
//! # Safety
//!
//! eBPF programs are verified by the kernel before loading. All memory
//! accesses are bounds-checked by the verifier.

#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{HashMap, XskMap},
    programs::XdpContext,
};
use aya_log_ebpf::info;

/// Connection ID to queue mapping
/// Key: Connection ID (8 bytes)
/// Value: Queue index for XSK redirect
#[map]
static CONNECTION_MAP: HashMap<[u8; 8], u32> = HashMap::with_max_entries(65536, 0);

/// AF_XDP socket map (one per RX queue)
#[map]
static XSK_MAP: XskMap = XskMap::with_max_entries(64, 0);

/// UDP port for protocol (configurable via userspace)
#[map]
static CONFIG: HashMap<u32, u32> = HashMap::with_max_entries(16, 0);

const CONFIG_KEY_PORT: u32 = 0;
const DEFAULT_PORT: u32 = 0; // 0 = all ports (protocol packets have unique CID)

/// XDP program entry point
#[xdp]
pub fn protocol_xdp(ctx: XdpContext) -> u32 {
    match try_protocol_xdp(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_PASS, // Pass unknown packets to kernel
    }
}

#[inline(always)]
fn try_protocol_xdp(ctx: &XdpContext) -> Result<u32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();
    
    // Parse Ethernet header (14 bytes)
    let eth_hdr = unsafe { ptr_at::<EthHdr>(data, 0, data_end)? };
    
    // Only process IPv4/IPv6
    let (ip_proto, ip_hdr_len) = match u16::from_be(eth_hdr.ether_type) {
        ETH_P_IP => {
            let ip_hdr = unsafe { ptr_at::<Ipv4Hdr>(data, 14, data_end)? };
            (ip_hdr.protocol, ((ip_hdr.ihl_version & 0x0F) * 4) as usize)
        }
        ETH_P_IPV6 => {
            let ip_hdr = unsafe { ptr_at::<Ipv6Hdr>(data, 14, data_end)? };
            (ip_hdr.next_header, 40)
        }
        _ => return Ok(xdp_action::XDP_PASS),
    };
    
    // Only process UDP
    if ip_proto != IPPROTO_UDP {
        return Ok(xdp_action::XDP_PASS);
    }
    
    // Parse UDP header
    let udp_offset = 14 + ip_hdr_len;
    let udp_hdr = unsafe { ptr_at::<UdpHdr>(data, udp_offset, data_end)? };
    
    // Check port if configured
    if let Some(&port) = unsafe { CONFIG.get(&CONFIG_KEY_PORT) } {
        if port != 0 && u16::from_be(udp_hdr.dest) != port as u16 {
            return Ok(xdp_action::XDP_PASS);
        }
    }
    
    // Parse Connection ID (first 8 bytes of UDP payload)
    let payload_offset = udp_offset + 8; // UDP header is 8 bytes
    let cid = unsafe { ptr_at::<[u8; 8]>(data, payload_offset, data_end)? };
    
    // Look up connection in map
    if let Some(&queue_idx) = unsafe { CONNECTION_MAP.get(cid) } {
        // Redirect to AF_XDP socket
        return Ok(XSK_MAP.redirect(queue_idx, 0).unwrap_or(xdp_action::XDP_PASS));
    }
    
    // Check for handshake CID (0xFFFFFFFFFFFFFFFF)
    if *cid == [0xFF; 8] {
        // Handshake packets go to queue 0
        return Ok(XSK_MAP.redirect(0, 0).unwrap_or(xdp_action::XDP_PASS));
    }
    
    // Unknown connection, pass to kernel stack
    Ok(xdp_action::XDP_PASS)
}

/// Safe pointer access with bounds checking (required by eBPF verifier)
#[inline(always)]
unsafe fn ptr_at<T>(data: usize, offset: usize, data_end: usize) -> Result<&'static T, ()> {
    let ptr = (data + offset) as *const T;
    if (ptr as usize) + core::mem::size_of::<T>() > data_end {
        return Err(());
    }
    Ok(&*ptr)
}

// Protocol constants
const ETH_P_IP: u16 = 0x0800;
const ETH_P_IPV6: u16 = 0x86DD;
const IPPROTO_UDP: u8 = 17;

// Header structures (packed, network byte order)
#[repr(C, packed)]
struct EthHdr {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ether_type: u16,
}

#[repr(C, packed)]
struct Ipv4Hdr {
    ihl_version: u8,
    tos: u8,
    tot_len: u16,
    id: u16,
    frag_off: u16,
    ttl: u8,
    protocol: u8,
    check: u16,
    src_addr: u32,
    dst_addr: u32,
}

#[repr(C, packed)]
struct Ipv6Hdr {
    priority_version: u8,
    flow_label: [u8; 3],
    payload_len: u16,
    next_header: u8,
    hop_limit: u8,
    src_addr: [u8; 16],
    dst_addr: [u8; 16],
}

#[repr(C, packed)]
struct UdpHdr {
    source: u16,
    dest: u16,
    len: u16,
    check: u16,
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

### 4.2 AF_XDP Socket Management (`protocol-transport/src/af_xdp.rs`)

```rust
//! AF_XDP socket management for zero-copy packet I/O.
//!
//! AF_XDP provides a kernel bypass path for packet processing, achieving
//! throughput of 10-40 Gbps on commodity hardware. This module handles
//! UMEM allocation, ring buffer management, and batch operations.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                          UMEM                                   │
//! │  (Shared memory region for packet buffers, 64MB typical)       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Fill Ring  │  Completion Ring  │  RX Ring  │  TX Ring         │
//! │  (buffers   │  (completed TX    │ (received │ (to transmit)   │
//! │   for RX)   │   buffers)        │  packets) │                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust
//! let config = XskConfig::default();
//! let socket = XskSocket::new("eth0", 0, config)?;
//!
//! // Receive packets
//! let packets = socket.recv_batch(64)?;
//! for pkt in packets {
//!     process_packet(&pkt);
//! }
//!
//! // Transmit packets
//! socket.send_batch(&outgoing)?;
//! ```

use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{io, ptr};

use libc::{
    c_void, mmap, munmap, setsockopt, socket,
    AF_XDP, MAP_ANONYMOUS, MAP_HUGETLB, MAP_PRIVATE, MAP_SHARED,
    PROT_READ, PROT_WRITE, SOCK_RAW, SOL_XDP,
};

/// AF_XDP socket configuration
#[derive(Debug, Clone)]
pub struct XskConfig {
    /// Frame size (must be power of 2, typically 4096)
    pub frame_size: u32,
    
    /// Number of frames in UMEM
    pub num_frames: u32,
    
    /// Fill ring size (power of 2)
    pub fill_ring_size: u32,
    
    /// Completion ring size (power of 2)
    pub comp_ring_size: u32,
    
    /// RX ring size (power of 2)
    pub rx_ring_size: u32,
    
    /// TX ring size (power of 2)
    pub tx_ring_size: u32,
    
    /// Use zero-copy mode (requires driver support)
    pub zero_copy: bool,
    
    /// Use huge pages for UMEM
    pub huge_pages: bool,
    
    /// Busy-poll budget (0 to disable)
    pub busy_poll_budget: u32,
}

impl Default for XskConfig {
    fn default() -> Self {
        Self {
            frame_size: 4096,
            num_frames: 16384,       // 64MB UMEM
            fill_ring_size: 4096,
            comp_ring_size: 4096,
            rx_ring_size: 4096,
            tx_ring_size: 4096,
            zero_copy: true,
            huge_pages: true,
            busy_poll_budget: 64,
        }
    }
}

/// UMEM region for packet buffers
struct Umem {
    /// Base address of mapped memory
    addr: *mut u8,
    
    /// Total size in bytes
    size: usize,
    
    /// Frame size
    frame_size: u32,
    
    /// Number of frames
    num_frames: u32,
    
    /// Free frame indices (lock-free stack)
    free_frames: Vec<AtomicU32>,
    
    /// Head of free list
    free_head: AtomicU32,
}

impl Umem {
    fn allocate(config: &XskConfig, numa_node: i32) -> io::Result<Self> {
        let size = config.frame_size as usize * config.num_frames as usize;
        
        // Allocate with huge pages if requested
        let mut flags = MAP_PRIVATE | MAP_ANONYMOUS;
        if config.huge_pages {
            flags |= MAP_HUGETLB;
        }
        
        let addr = unsafe {
            let ptr = mmap(
                ptr::null_mut(),
                size,
                PROT_READ | PROT_WRITE,
                flags,
                -1,
                0,
            );
            
            if ptr == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }
            
            // Bind to NUMA node if specified
            if numa_node >= 0 {
                let nodemask: u64 = 1 << numa_node;
                libc::mbind(
                    ptr,
                    size,
                    libc::MPOL_BIND,
                    &nodemask as *const _ as *const _,
                    64,
                    libc::MPOL_MF_MOVE,
                );
            }
            
            ptr as *mut u8
        };
        
        // Initialize free list
        let mut free_frames = Vec::with_capacity(config.num_frames as usize);
        for i in 0..config.num_frames {
            free_frames.push(AtomicU32::new(i));
        }
        
        Ok(Self {
            addr,
            size,
            frame_size: config.frame_size,
            num_frames: config.num_frames,
            free_frames,
            free_head: AtomicU32::new(config.num_frames),
        })
    }
    
    /// Allocate a frame from the free list
    fn alloc_frame(&self) -> Option<u32> {
        loop {
            let head = self.free_head.load(Ordering::Acquire);
            if head == 0 {
                return None; // No free frames
            }
            
            let new_head = head - 1;
            if self.free_head.compare_exchange_weak(
                head, new_head, Ordering::AcqRel, Ordering::Relaxed
            ).is_ok() {
                return Some(self.free_frames[new_head as usize].load(Ordering::Relaxed));
            }
        }
    }
    
    /// Return a frame to the free list
    fn free_frame(&self, frame_idx: u32) {
        loop {
            let head = self.free_head.load(Ordering::Acquire);
            self.free_frames[head as usize].store(frame_idx, Ordering::Relaxed);
            
            if self.free_head.compare_exchange_weak(
                head, head + 1, Ordering::AcqRel, Ordering::Relaxed
            ).is_ok() {
                return;
            }
        }
    }
    
    /// Get frame address from index
    fn frame_addr(&self, idx: u32) -> *mut u8 {
        unsafe { self.addr.add((idx * self.frame_size) as usize) }
    }
}

impl Drop for Umem {
    fn drop(&mut self) {
        unsafe {
            munmap(self.addr as *mut c_void, self.size);
        }
    }
}

/// Ring buffer for AF_XDP
struct XskRing {
    /// Producer index (written by producer)
    producer: *mut AtomicU32,
    
    /// Consumer index (written by consumer)  
    consumer: *mut AtomicU32,
    
    /// Ring entries
    ring: *mut u64,
    
    /// Ring mask (size - 1)
    mask: u32,
    
    /// Cached producer value (for batching)
    cached_producer: u32,
    
    /// Cached consumer value (for batching)
    cached_consumer: u32,
}

/// AF_XDP socket
pub struct XskSocket {
    /// Socket file descriptor
    fd: RawFd,
    
    /// UMEM region
    umem: Umem,
    
    /// Fill ring (userspace → kernel: empty buffers for RX)
    fill_ring: XskRing,
    
    /// Completion ring (kernel → userspace: completed TX buffers)
    comp_ring: XskRing,
    
    /// RX ring (kernel → userspace: received packets)
    rx_ring: XskRing,
    
    /// TX ring (userspace → kernel: packets to transmit)
    tx_ring: XskRing,
    
    /// Interface name
    ifname: String,
    
    /// Queue ID
    queue_id: u32,
    
    /// Configuration
    config: XskConfig,
}

/// Received packet reference
pub struct RxPacket<'a> {
    /// Packet data slice
    pub data: &'a [u8],
    
    /// Frame index (for returning to UMEM)
    frame_idx: u32,
    
    /// Reference to socket for returning frame
    socket: &'a XskSocket,
}

impl<'a> Drop for RxPacket<'a> {
    fn drop(&mut self) {
        self.socket.umem.free_frame(self.frame_idx);
    }
}

impl XskSocket {
    /// Create a new AF_XDP socket
    ///
    /// # Arguments
    /// * `ifname` - Network interface name (e.g., "eth0")
    /// * `queue_id` - NIC queue index
    /// * `config` - Socket configuration
    ///
    /// # Returns
    /// * `Ok(XskSocket)` - Configured socket ready for I/O
    /// * `Err` - Setup failure
    pub fn new(ifname: &str, queue_id: u32, config: XskConfig) -> io::Result<Self> {
        // Create AF_XDP socket
        let fd = unsafe { socket(AF_XDP, SOCK_RAW, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        
        // Get NUMA node for interface
        let numa_node = get_interface_numa_node(ifname).unwrap_or(-1);
        
        // Allocate UMEM
        let umem = Umem::allocate(&config, numa_node)?;
        
        // Register UMEM with kernel
        // ... (XDP_UMEM_REG, XDP_UMEM_FILL_RING, etc.)
        
        // Set up rings
        // ... (mmap ring memory, initialize pointers)
        
        // Bind to interface/queue
        // ... (bind() with sockaddr_xdp)
        
        // Pre-fill the fill ring with empty buffers
        // ... 
        
        Ok(Self {
            fd,
            umem,
            fill_ring: todo!(),
            comp_ring: todo!(),
            rx_ring: todo!(),
            tx_ring: todo!(),
            ifname: ifname.to_string(),
            queue_id,
            config,
        })
    }
    
    /// Receive a batch of packets
    ///
    /// # Arguments
    /// * `max_packets` - Maximum packets to receive
    ///
    /// # Returns
    /// * Vector of received packet references
    pub fn recv_batch(&self, max_packets: usize) -> io::Result<Vec<RxPacket<'_>>> {
        let mut packets = Vec::with_capacity(max_packets);
        
        // Read from RX ring
        // ... (check producer/consumer indices, extract descriptors)
        
        // Refill the fill ring with returned buffers
        // ...
        
        Ok(packets)
    }
    
    /// Transmit a batch of packets
    ///
    /// # Arguments
    /// * `packets` - Packet data to transmit
    ///
    /// # Returns
    /// * Number of packets queued for transmission
    pub fn send_batch(&self, packets: &[&[u8]]) -> io::Result<usize> {
        let mut sent = 0;
        
        // Reclaim completed TX buffers
        // ... (read completion ring)
        
        // Write to TX ring
        for pkt in packets {
            if let Some(frame_idx) = self.umem.alloc_frame() {
                // Copy packet to UMEM frame
                let frame_addr = self.umem.frame_addr(frame_idx);
                unsafe {
                    ptr::copy_nonoverlapping(pkt.as_ptr(), frame_addr, pkt.len());
                }
                
                // Add descriptor to TX ring
                // ...
                
                sent += 1;
            } else {
                break; // No more free frames
            }
        }
        
        // Kick the kernel if needed
        if sent > 0 {
            self.kick_tx()?;
        }
        
        Ok(sent)
    }
    
    /// Signal kernel to process TX ring
    fn kick_tx(&self) -> io::Result<()> {
        // sendto() with MSG_DONTWAIT on AF_XDP socket
        unsafe {
            let ret = libc::sendto(
                self.fd,
                ptr::null(),
                0,
                libc::MSG_DONTWAIT,
                ptr::null(),
                0,
            );
            if ret < 0 {
                let err = io::Error::last_os_error();
                if err.kind() != io::ErrorKind::WouldBlock {
                    return Err(err);
                }
            }
        }
        Ok(())
    }
    
    /// Get file descriptor for polling
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for XskSocket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Get NUMA node for a network interface
fn get_interface_numa_node(ifname: &str) -> Option<i32> {
    let path = format!("/sys/class/net/{}/device/numa_node", ifname);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}
```

### 4.3 io_uring Integration (`protocol-transport/src/io_uring.rs`)

```rust
//! io_uring integration for high-performance async file I/O.
//!
//! io_uring provides the most efficient async I/O mechanism on Linux,
//! with support for zero-copy operations and batched syscalls. This
//! module wraps io_uring for file reading/writing during transfers.
//!
//! # Features
//!
//! - Zero-copy send with IORING_OP_SEND_ZC (kernel 6.0+)
//! - Multishot receive for reduced syscall overhead
//! - Registered buffers for faster I/O
//! - SQE linking for dependent operations

use std::fs::File;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;

use io_uring::{opcode, squeue, types, IoUring, Probe};

/// io_uring configuration
#[derive(Debug, Clone)]
pub struct UringConfig {
    /// Submission queue depth
    pub sq_depth: u32,
    
    /// Completion queue depth (typically 2x SQ)
    pub cq_depth: u32,
    
    /// Use SQPOLL mode (kernel-side polling)
    pub sqpoll: bool,
    
    /// SQPOLL idle timeout (ms)
    pub sqpoll_idle: u32,
    
    /// Use registered buffers
    pub registered_buffers: bool,
    
    /// Number of registered buffers
    pub num_buffers: usize,
    
    /// Size of each registered buffer
    pub buffer_size: usize,
}

impl Default for UringConfig {
    fn default() -> Self {
        Self {
            sq_depth: 256,
            cq_depth: 512,
            sqpoll: false,       // Requires root/CAP_SYS_NICE
            sqpoll_idle: 1000,
            registered_buffers: true,
            num_buffers: 64,
            buffer_size: 262144, // 256 KiB (chunk size)
        }
    }
}

/// File I/O manager using io_uring
pub struct FileIoManager {
    /// io_uring instance
    ring: IoUring,
    
    /// Registered file descriptors
    registered_fds: Vec<RawFd>,
    
    /// Registered buffers
    buffers: Option<RegisteredBuffers>,
    
    /// Pending operations (user_data → callback)
    pending: std::collections::HashMap<u64, PendingOp>,
    
    /// Next user_data value
    next_user_data: u64,
    
    /// Configuration
    config: UringConfig,
}

/// Registered buffer pool
struct RegisteredBuffers {
    /// Buffer memory (contiguous allocation)
    memory: Vec<u8>,
    
    /// Buffer size
    buffer_size: usize,
    
    /// Number of buffers
    count: usize,
    
    /// Free buffer indices
    free: Vec<usize>,
}

impl RegisteredBuffers {
    fn new(count: usize, buffer_size: usize) -> Self {
        let memory = vec![0u8; count * buffer_size];
        let free = (0..count).collect();
        
        Self {
            memory,
            buffer_size,
            count,
            free,
        }
    }
    
    fn alloc(&mut self) -> Option<usize> {
        self.free.pop()
    }
    
    fn free(&mut self, idx: usize) {
        self.free.push(idx);
    }
    
    fn get_slice(&self, idx: usize) -> &[u8] {
        let start = idx * self.buffer_size;
        &self.memory[start..start + self.buffer_size]
    }
    
    fn get_slice_mut(&mut self, idx: usize) -> &mut [u8] {
        let start = idx * self.buffer_size;
        &mut self.memory[start..start + self.buffer_size]
    }
}

/// Pending operation callback
enum PendingOp {
    Read {
        callback: Box<dyn FnOnce(io::Result<usize>) + Send>,
        buffer_idx: Option<usize>,
    },
    Write {
        callback: Box<dyn FnOnce(io::Result<usize>) + Send>,
    },
    Fsync {
        callback: Box<dyn FnOnce(io::Result<()>) + Send>,
    },
}

impl FileIoManager {
    /// Create a new file I/O manager
    pub fn new(config: UringConfig) -> io::Result<Self> {
        // Check kernel support
        let probe = Probe::new()?;
        
        // Build io_uring with requested features
        let mut builder = IoUring::builder();
        builder.dontfork();
        
        if config.sqpoll {
            builder.setup_sqpoll(config.sqpoll_idle);
        }
        
        // Use cooperative taskrun for better batching (kernel 5.19+)
        if probe.is_supported(opcode::types::IORING_OP_NOP) {
            builder.setup_coop_taskrun();
            builder.setup_single_issuer();
        }
        
        let ring = builder.build(config.sq_depth)?;
        
        // Set up registered buffers
        let buffers = if config.registered_buffers {
            let bufs = RegisteredBuffers::new(config.num_buffers, config.buffer_size);
            
            // Register with kernel
            let iovecs: Vec<_> = (0..bufs.count)
                .map(|i| {
                    let slice = bufs.get_slice(i);
                    types::IoSlice::new(slice)
                })
                .collect();
            
            // ring.submitter().register_buffers(&iovecs)?;
            
            Some(bufs)
        } else {
            None
        };
        
        Ok(Self {
            ring,
            registered_fds: Vec::new(),
            buffers,
            pending: std::collections::HashMap::new(),
            next_user_data: 1,
            config,
        })
    }
    
    /// Register a file for faster operations
    pub fn register_file(&mut self, file: &File) -> io::Result<u32> {
        let fd = file.as_raw_fd();
        let idx = self.registered_fds.len() as u32;
        self.registered_fds.push(fd);
        
        // Re-register all files (io_uring limitation)
        self.ring.submitter().register_files(&self.registered_fds)?;
        
        Ok(idx)
    }
    
    /// Submit an async read operation
    ///
    /// # Arguments
    /// * `file_idx` - Registered file index
    /// * `offset` - File offset to read from
    /// * `len` - Number of bytes to read
    /// * `callback` - Called with result when complete
    pub fn read_async<F>(
        &mut self,
        file_idx: u32,
        offset: u64,
        len: usize,
        callback: F,
    ) -> io::Result<()>
    where
        F: FnOnce(io::Result<(usize, Vec<u8>)>) + Send + 'static,
    {
        let user_data = self.next_user_data;
        self.next_user_data += 1;
        
        // Allocate buffer
        let (buffer_idx, buf_ptr, buf_len) = if let Some(ref mut bufs) = self.buffers {
            if let Some(idx) = bufs.alloc() {
                let slice = bufs.get_slice_mut(idx);
                let len = len.min(slice.len());
                (Some(idx), slice.as_mut_ptr(), len)
            } else {
                // Fallback to heap allocation
                let mut buf = vec![0u8; len];
                (None, buf.as_mut_ptr(), len)
            }
        } else {
            let mut buf = vec![0u8; len];
            (None, buf.as_mut_ptr(), len)
        };
        
        // Build SQE
        let sqe = opcode::Read::new(
            types::Fd(self.registered_fds[file_idx as usize]),
            buf_ptr,
            buf_len as u32,
        )
        .offset(offset)
        .build()
        .user_data(user_data);
        
        // Submit
        unsafe {
            self.ring.submission().push(&sqe)?;
        }
        
        // Track pending op
        self.pending.insert(user_data, PendingOp::Read {
            callback: Box::new(move |result| {
                // Handle result and call user callback
                // ...
            }),
            buffer_idx,
        });
        
        Ok(())
    }
    
    /// Submit an async write operation
    pub fn write_async<F>(
        &mut self,
        file_idx: u32,
        offset: u64,
        data: &[u8],
        callback: F,
    ) -> io::Result<()>
    where
        F: FnOnce(io::Result<usize>) + Send + 'static,
    {
        let user_data = self.next_user_data;
        self.next_user_data += 1;
        
        // Build SQE
        let sqe = opcode::Write::new(
            types::Fd(self.registered_fds[file_idx as usize]),
            data.as_ptr(),
            data.len() as u32,
        )
        .offset(offset)
        .build()
        .user_data(user_data);
        
        // Submit
        unsafe {
            self.ring.submission().push(&sqe)?;
        }
        
        self.pending.insert(user_data, PendingOp::Write {
            callback: Box::new(callback),
        });
        
        Ok(())
    }
    
    /// Process completed operations
    ///
    /// # Arguments
    /// * `wait` - Block until at least one completion
    ///
    /// # Returns
    /// * Number of completions processed
    pub fn poll_completions(&mut self, wait: bool) -> io::Result<usize> {
        if wait {
            self.ring.submit_and_wait(1)?;
        } else {
            self.ring.submit()?;
        }
        
        let mut count = 0;
        
        while let Some(cqe) = self.ring.completion().next() {
            let user_data = cqe.user_data();
            let result = cqe.result();
            
            if let Some(op) = self.pending.remove(&user_data) {
                match op {
                    PendingOp::Read { callback, buffer_idx } => {
                        if result >= 0 {
                            callback(Ok(result as usize));
                        } else {
                            callback(Err(io::Error::from_raw_os_error(-result)));
                        }
                        
                        // Return buffer to pool
                        if let (Some(idx), Some(ref mut bufs)) = (buffer_idx, &mut self.buffers) {
                            bufs.free(idx);
                        }
                    }
                    PendingOp::Write { callback } => {
                        if result >= 0 {
                            callback(Ok(result as usize));
                        } else {
                            callback(Err(io::Error::from_raw_os_error(-result)));
                        }
                    }
                    PendingOp::Fsync { callback } => {
                        if result >= 0 {
                            callback(Ok(()));
                        } else {
                            callback(Err(io::Error::from_raw_os_error(-result)));
                        }
                    }
                }
            }
            
            count += 1;
        }
        
        Ok(count)
    }
}
```

---

## 5. Cryptographic Implementation

### 5.1 Noise Protocol Handshake (`protocol-crypto/src/noise.rs`)

```rust
//! Noise_XX handshake implementation with Elligator2 encoding.
//!
//! This module implements the Noise Protocol Framework's XX pattern,
//! providing mutual authentication and forward secrecy. All public
//! keys are encoded using Elligator2 to appear as random data.
//!
//! # Security Properties
//!
//! - Mutual authentication (both parties verified)
//! - Forward secrecy (compromise doesn't reveal past sessions)
//! - Identity hiding (static keys encrypted)
//! - Indistinguishability (keys look like random bytes)

use snow::{Builder, HandshakeState, TransportState};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::elligator::Elligator2;
use crate::CryptoError;

/// Noise protocol pattern string
const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// Noise session for handshake
pub struct NoiseSession {
    /// Snow handshake state
    state: HandshakeState,
    
    /// Our static key pair
    static_key: StaticSecret,
    
    /// Whether we are the initiator
    is_initiator: bool,
    
    /// Elligator2 encoder
    elligator: Elligator2,
}

impl NoiseSession {
    /// Create a new session as initiator
    ///
    /// # Arguments
    /// * `remote_static_public` - Remote peer's static public key (32 bytes)
    pub fn new_initiator(remote_static_public: &[u8; 32]) -> Result<Self, CryptoError> {
        let static_key = StaticSecret::random();
        
        let state = Builder::new(NOISE_PATTERN.parse()?)
            .local_private_key(&static_key.to_bytes())
            .remote_public_key(remote_static_public)
            .build_initiator()?;
        
        Ok(Self {
            state,
            static_key,
            is_initiator: true,
            elligator: Elligator2::new(),
        })
    }
    
    /// Create a new session as responder
    ///
    /// # Arguments
    /// * `local_static_key` - Our static private key (32 bytes)
    pub fn new_responder(local_static_key: &[u8; 32]) -> Result<Self, CryptoError> {
        let static_key = StaticSecret::from(*local_static_key);
        
        let state = Builder::new(NOISE_PATTERN.parse()?)
            .local_private_key(&static_key.to_bytes())
            .build_responder()?;
        
        Ok(Self {
            state,
            static_key,
            is_initiator: false,
            elligator: Elligator2::new(),
        })
    }
    
    /// Write handshake message 1 (initiator → responder)
    pub fn write_message_1(&mut self) -> Result<Vec<u8>, CryptoError> {
        let mut buf = vec![0u8; 128];
        let len = self.state.write_message(&[], &mut buf)?;
        buf.truncate(len);
        
        // Encode ephemeral public key with Elligator2
        self.encode_ephemeral(&mut buf[..32])?;
        
        Ok(buf)
    }
    
    /// Read handshake message 1 (responder receives)
    pub fn read_message_1(&mut self, message: &[u8]) -> Result<(), CryptoError> {
        // Decode ephemeral public key from Elligator2
        let mut decoded = message.to_vec();
        self.decode_ephemeral(&mut decoded[..32])?;
        
        let mut payload = vec![0u8; 64];
        self.state.read_message(&decoded, &mut payload)?;
        
        Ok(())
    }
    
    /// Write handshake message 2 (responder → initiator)
    pub fn write_message_2(&mut self) -> Result<Vec<u8>, CryptoError> {
        let mut buf = vec![0u8; 128];
        let len = self.state.write_message(&[], &mut buf)?;
        buf.truncate(len);
        
        // Encode our ephemeral with Elligator2
        self.encode_ephemeral(&mut buf[..32])?;
        
        Ok(buf)
    }
    
    /// Read handshake message 2 (initiator receives)
    pub fn read_message_2(&mut self, message: &[u8]) -> Result<(), CryptoError> {
        let mut decoded = message.to_vec();
        self.decode_ephemeral(&mut decoded[..32])?;
        
        let mut payload = vec![0u8; 64];
        self.state.read_message(&decoded, &mut payload)?;
        
        Ok(())
    }
    
    /// Write handshake message 3 (initiator → responder)
    pub fn write_message_3(&mut self) -> Result<Vec<u8>, CryptoError> {
        let mut buf = vec![0u8; 128];
        let len = self.state.write_message(&[], &mut buf)?;
        buf.truncate(len);
        
        Ok(buf)
    }
    
    /// Read handshake message 3 (responder receives)
    pub fn read_message_3(&mut self, message: &[u8]) -> Result<(), CryptoError> {
        let mut payload = vec![0u8; 64];
        self.state.read_message(message, &mut payload)?;
        
        Ok(())
    }
    
    /// Convert to transport mode and extract session keys
    pub fn into_keys(self) -> Result<SessionKeys, CryptoError> {
        let transport = self.state.into_transport_mode()?;
        SessionKeys::from_transport(transport, self.is_initiator)
    }
    
    /// Encode a public key point using Elligator2
    fn encode_ephemeral(&self, key: &mut [u8]) -> Result<(), CryptoError> {
        // Convert X25519 point to Elligator2 representative
        self.elligator.encode_point(key)
    }
    
    /// Decode an Elligator2 representative to a public key
    fn decode_ephemeral(&self, repr: &mut [u8]) -> Result<(), CryptoError> {
        // Convert Elligator2 representative to X25519 point
        self.elligator.decode_representative(repr)
    }
}

/// Session keys derived from completed handshake
#[derive(ZeroizeOnDrop)]
pub struct SessionKeys {
    /// Key for encrypting data we send
    send_key: [u8; 32],
    
    /// Key for decrypting data we receive
    recv_key: [u8; 32],
    
    /// Chain key for forward secrecy ratchet
    chain_key: [u8; 32],
    
    /// Nonce counter for send direction
    send_nonce: u64,
    
    /// Nonce counter for receive direction
    recv_nonce: u64,
    
    /// Transport state for rekeying
    transport: TransportState,
}

impl SessionKeys {
    fn from_transport(transport: TransportState, is_initiator: bool) -> Result<Self, CryptoError> {
        // Extract cipher states
        let (send_cipher, recv_cipher) = if is_initiator {
            transport.get_ciphers()
        } else {
            let (a, b) = transport.get_ciphers();
            (b, a)
        };
        
        // Derive keys using HKDF
        let mut send_key = [0u8; 32];
        let mut recv_key = [0u8; 32];
        let mut chain_key = [0u8; 32];
        
        // ... key derivation ...
        
        Ok(Self {
            send_key,
            recv_key,
            chain_key,
            send_nonce: 0,
            recv_nonce: 0,
            transport,
        })
    }
    
    /// Encrypt a frame
    pub fn encrypt(&mut self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        use chacha20poly1305::{XChaCha20Poly1305, aead::{Aead, KeyInit}};
        
        let cipher = XChaCha20Poly1305::new_from_slice(&self.send_key)?;
        
        // Build nonce (24 bytes for XChaCha20)
        let mut nonce = [0u8; 24];
        nonce[16..].copy_from_slice(&self.send_nonce.to_be_bytes());
        self.send_nonce += 1;
        
        let ciphertext = cipher.encrypt(&nonce.into(), plaintext)?;
        
        // Ratchet chain key
        self.ratchet_send_key();
        
        Ok(ciphertext)
    }
    
    /// Decrypt a frame
    pub fn decrypt(&mut self, ciphertext: &[u8], nonce: &[u8; 8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        use chacha20poly1305::{XChaCha20Poly1305, aead::{Aead, KeyInit}};
        
        let cipher = XChaCha20Poly1305::new_from_slice(&self.recv_key)?;
        
        // Build full nonce
        let mut full_nonce = [0u8; 24];
        full_nonce[16..].copy_from_slice(nonce);
        
        let plaintext = cipher.decrypt(&full_nonce.into(), ciphertext)?;
        
        // Ratchet chain key
        self.ratchet_recv_key();
        
        Ok(plaintext)
    }
    
    /// Derive connection ID from session keys
    pub fn derive_connection_id(&self) -> [u8; 8] {
        use blake3::Hasher;
        
        let mut hasher = Hasher::new_derive_key("connection-id");
        hasher.update(&self.chain_key);
        
        let mut cid = [0u8; 8];
        cid.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
        cid
    }
    
    /// Create a rekey message for forward secrecy
    pub fn create_rekey_message(&mut self) -> Result<Vec<u8>, CryptoError> {
        // Generate new ephemeral key pair
        let new_ephemeral = StaticSecret::random();
        let new_public = PublicKey::from(&new_ephemeral);
        
        // Encode with Elligator2
        let mut message = new_public.to_bytes().to_vec();
        // ... Elligator2 encoding ...
        
        Ok(message)
    }
    
    fn ratchet_send_key(&mut self) {
        use blake3::Hasher;
        
        let mut hasher = Hasher::new_derive_key("send-ratchet");
        hasher.update(&self.chain_key);
        hasher.update(&[0x01]);
        
        let output = hasher.finalize();
        self.chain_key.copy_from_slice(&output.as_bytes()[..32]);
        
        let mut hasher = Hasher::new_derive_key("send-key");
        hasher.update(&self.chain_key);
        hasher.update(&[0x02]);
        
        let output = hasher.finalize();
        self.send_key.copy_from_slice(&output.as_bytes()[..32]);
    }
    
    fn ratchet_recv_key(&mut self) {
        // Similar to send key ratchet
    }
}
```

---

## 6. Transport Layer Implementation

*[Sections 6-13 would continue with similar depth, covering:]*

- **Section 6:** Worker loop implementation, packet processing pipeline
- **Section 7:** File chunking, BLAKE3 tree hashing, transfer state machine
- **Section 8:** DHT integration, relay client, NAT traversal implementation
- **Section 9:** Public API traits and types
- **Section 10:** Unit tests, integration tests, fuzzing
- **Section 11:** CPU pinning, NUMA optimization, profiling
- **Section 12:** Cargo features, cross-compilation, systemd integration
- **Section 13:** Sandboxing, capability dropping, seccomp filters

---

## 9. API Design

### 9.1 Core Public API

```rust
//! Public API for the decentralized file transfer protocol.
//!
//! # Quick Start
//!
//! ```rust
//! use protocol::{Node, NodeConfig, Transfer};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a node
//!     let config = NodeConfig::default();
//!     let node = Node::new(config).await?;
//!     
//!     // Send a file
//!     let transfer = node.send_file(
//!         "path/to/file.dat",
//!         "peer_public_key_hex",
//!     ).await?;
//!     
//!     // Wait for completion
//!     transfer.await_completion().await?;
//!     
//!     Ok(())
//! }
//! ```

/// Node configuration
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Network interface to use (None = auto-detect)
    pub interface: Option<String>,
    
    /// UDP port to listen on (0 = random)
    pub port: u16,
    
    /// Path to identity key file
    pub identity_path: PathBuf,
    
    /// Enable XDP acceleration
    pub enable_xdp: bool,
    
    /// Maximum concurrent transfers
    pub max_transfers: usize,
    
    /// Obfuscation mode
    pub obfuscation: ObfuscationMode,
    
    /// DHT bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
    
    /// Relay servers
    pub relay_servers: Vec<String>,
}

/// Obfuscation mode for traffic hiding
#[derive(Debug, Clone, Copy)]
pub enum ObfuscationMode {
    /// No obfuscation (fastest)
    None,
    
    /// Basic padding and timing
    Basic,
    
    /// Full obfuscation with cover traffic
    Full,
    
    /// Protocol mimicry (HTTPS, WebSocket, etc.)
    Mimicry(MimicryType),
}

/// Protocol to mimic
#[derive(Debug, Clone, Copy)]
pub enum MimicryType {
    Https,
    WebSocket,
    DnsOverHttps,
}

/// A node in the decentralized network
pub struct Node {
    // ... internal fields ...
}

impl Node {
    /// Create a new node with the given configuration
    pub async fn new(config: NodeConfig) -> Result<Self, Error>;
    
    /// Get our public identity key
    pub fn public_key(&self) -> &[u8; 32];
    
    /// Send a file to a peer
    pub async fn send_file(
        &self,
        path: impl AsRef<Path>,
        recipient: &str,
    ) -> Result<Transfer, Error>;
    
    /// Receive files (returns receiver for incoming transfers)
    pub fn receive(&self) -> TransferReceiver;
    
    /// Connect to a specific peer
    pub async fn connect(&self, peer_key: &str) -> Result<Session, Error>;
    
    /// Announce availability of a file (for sharing)
    pub async fn announce_file(
        &self,
        path: impl AsRef<Path>,
        group_key: &str,
    ) -> Result<Announcement, Error>;
    
    /// Search for a file in the network
    pub async fn search_file(
        &self,
        file_hash: &str,
        group_key: &str,
    ) -> Result<Vec<PeerInfo>, Error>;
    
    /// Shutdown the node gracefully
    pub async fn shutdown(self) -> Result<(), Error>;
}

/// Active file transfer
pub struct Transfer {
    // ... internal fields ...
}

impl Transfer {
    /// Get transfer ID
    pub fn id(&self) -> TransferId;
    
    /// Get current progress (bytes transferred)
    pub fn progress(&self) -> u64;
    
    /// Get total size
    pub fn total_size(&self) -> u64;
    
    /// Get current transfer rate (bytes/sec)
    pub fn rate(&self) -> u64;
    
    /// Get estimated time remaining
    pub fn eta(&self) -> Option<Duration>;
    
    /// Wait for transfer completion
    pub async fn await_completion(self) -> Result<TransferResult, Error>;
    
    /// Cancel the transfer
    pub async fn cancel(self) -> Result<(), Error>;
    
    /// Pause the transfer
    pub async fn pause(&self) -> Result<(), Error>;
    
    /// Resume a paused transfer
    pub async fn resume(&self) -> Result<(), Error>;
}

/// Transfer result
pub struct TransferResult {
    /// Final file path
    pub path: PathBuf,
    
    /// File hash (BLAKE3)
    pub hash: [u8; 32],
    
    /// Total bytes transferred
    pub bytes: u64,
    
    /// Transfer duration
    pub duration: Duration,
    
    /// Average transfer rate
    pub average_rate: u64,
}
```

---

## 12. Build and Deployment

### 12.1 Build Requirements

```bash
# System dependencies (Ubuntu 24.04 / Debian 13)
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libelf-dev \
    clang \
    llvm \
    libbpf-dev \
    linux-headers-$(uname -r)

# Rust toolchain
rustup install stable
rustup component add rust-src  # For eBPF compilation

# eBPF toolchain
cargo install bpf-linker
```

### 12.2 Build Commands

```bash
# Build all crates (debug)
cargo build

# Build optimized release
cargo build --release

# Build with XDP support
cargo build --release --features xdp

# Build CLI only
cargo build --release -p protocol-cli

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench
```

### 12.3 Systemd Service

```ini
# /etc/systemd/system/protocol.service
[Unit]
Description=Decentralized File Transfer Protocol Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
ExecStart=/usr/local/bin/protocol serve --config /etc/protocol/config.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=5

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
PrivateDevices=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictNamespaces=true
RestrictRealtime=true
RestrictSUIDSGID=true
MemoryDenyWriteExecute=true
LockPersonality=true

# Required capabilities for XDP
AmbientCapabilities=CAP_NET_ADMIN CAP_BPF CAP_PERFMON
CapabilityBoundingSet=CAP_NET_ADMIN CAP_BPF CAP_PERFMON

[Install]
WantedBy=multi-user.target
```

---

*End of Protocol Implementation Guide*
