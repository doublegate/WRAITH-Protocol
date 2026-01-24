# WRAITH Protocol v2 API Changes

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Planning Document
**Authors:** WRAITH Protocol Team

---

## Table of Contents

1. [Overview](#overview)
2. [Breaking Changes](#breaking-changes)
3. [New APIs](#new-apis)
4. [Deprecated APIs](#deprecated-apis)
5. [Crate-by-Crate Changes](#crate-by-crate-changes)
6. [Migration Examples](#migration-examples)
7. [Compatibility Shims](#compatibility-shims)

---

## Overview

This document catalogs all API changes between WRAITH Protocol v1 and v2, including breaking changes, new additions, and deprecations.

### Change Categories

| Category | Symbol | Description |
|----------|--------|-------------|
| Breaking | **B** | Requires code changes |
| Additive | **A** | New functionality, backward compatible |
| Deprecated | **D** | Still works, will be removed |
| Removed | **R** | No longer available |
| Modified | **M** | Signature or behavior changed |

### Summary Statistics

| Crate | Breaking | Additive | Deprecated | Removed |
|-------|----------|----------|------------|---------|
| wraith-core | 12 | 35 | 5 | 3 |
| wraith-crypto | 8 | 25 | 3 | 2 |
| wraith-transport | 15 | 40 | 2 | 4 |
| wraith-obfuscation | 6 | 20 | 1 | 1 |
| wraith-discovery | 4 | 15 | 2 | 1 |
| wraith-files | 3 | 12 | 1 | 0 |
| wraith-cli | 5 | 18 | 3 | 2 |

---

## Breaking Changes

### Core Types

#### ConnectionId (B)

```rust
// v1: 64-bit connection ID
pub struct ConnectionId(u64);

impl ConnectionId {
    pub fn new() -> Self;
    pub fn from_bytes(bytes: [u8; 8]) -> Self;
    pub fn to_bytes(&self) -> [u8; 8];
}

// v2: 128-bit connection ID
pub struct ConnectionId {
    bytes: [u8; 16],
}

impl ConnectionId {
    pub fn generate() -> Self;
    pub fn from_bytes(bytes: [u8; 16]) -> Self;  // BREAKING: different size
    pub fn as_bytes(&self) -> &[u8; 16];         // BREAKING: renamed, different size

    // New methods
    pub fn from_v1(v1_cid: u64) -> Self;
    pub fn is_migrated_v1(&self) -> bool;
    pub fn original_v1(&self) -> Option<u64>;
}
```

**Migration:**
```rust
// Before
let cid = ConnectionId::new();
let bytes: [u8; 8] = cid.to_bytes();

// After
let cid = ConnectionId::generate();
let bytes: [u8; 16] = *cid.as_bytes();

// Or for migration
let v1_cid: u64 = old_cid.to_bytes().try_into().unwrap();
let v2_cid = ConnectionId::from_v1(v1_cid);
```

#### FrameHeader (B)

```rust
// v1: 20-byte header
pub struct FrameHeader {
    pub version: u8,        // Actually 4 bits
    pub frame_type: u8,     // Actually 4 bits
    pub flags: u8,
    pub sequence: u32,
    pub length: u16,
    pub connection_id: u64,
}

// v2: 24-byte header with expanded fields
pub struct FrameHeader {
    pub version: u8,           // Full byte
    pub frame_type: FrameType, // Type-safe enum
    pub flags: Flags,          // Bitflags type
    pub sequence: u64,         // BREAKING: expanded
    pub length: u32,           // BREAKING: expanded
    pub connection_id: ConnectionId,  // BREAKING: 128-bit
}
```

#### Session (B)

```rust
// v1: Simple constructor
impl Session {
    pub fn new(
        connection_id: ConnectionId,
        peer_addr: SocketAddr,
        shared_secret: [u8; 32],
    ) -> Result<Self>;

    pub fn send(&self, data: &[u8]) -> Result<()>;
    pub async fn send_async(&self, data: &[u8]) -> Result<()>;
}

// v2: Builder pattern with async-first API
impl Session {
    pub fn builder() -> SessionBuilder;

    // Async-first
    pub async fn send(&self, data: &[u8]) -> Result<()>;

    // Sync wrapper (not preferred)
    pub fn send_blocking(&self, data: &[u8]) -> Result<()>;
}

pub struct SessionBuilder {
    // ...
}

impl SessionBuilder {
    pub fn connection_id(self, cid: ConnectionId) -> Self;
    pub fn peer_addr(self, addr: SocketAddr) -> Self;
    pub fn crypto_context(self, ctx: CryptoContext) -> Self;  // NEW
    pub fn wire_format(self, format: WireFormat) -> Self;     // NEW
    pub fn transport(self, transport: TransportHandle) -> Self;  // NEW
    pub fn build(self) -> Result<Session>;
}
```

**Migration:**
```rust
// Before
let session = Session::new(cid, addr, secret)?;
session.send(data)?;

// After
let session = Session::builder()
    .connection_id(cid)
    .peer_addr(addr)
    .crypto_context(CryptoContext::from_classical(secret))
    .build()?;
session.send(data).await?;  // Now async
```

### Cryptographic Types

#### SharedSecret (B)

```rust
// v1: Simple wrapper
pub struct SharedSecret([u8; 32]);

impl SharedSecret {
    pub fn from_bytes(bytes: [u8; 32]) -> Self;
    pub fn as_bytes(&self) -> &[u8; 32];
}

// v2: Zeroizing wrapper with source tracking
pub struct SharedSecret {
    bytes: Zeroizing<[u8; 32]>,
    source: SecretSource,
}

impl SharedSecret {
    pub fn from_classical(bytes: [u8; 32]) -> Self;
    pub fn from_hybrid(classical: [u8; 32], post_quantum: &[u8]) -> Self;
    pub fn as_bytes(&self) -> &[u8; 32];
    pub fn is_hybrid(&self) -> bool;
    pub fn source(&self) -> SecretSource;
}

pub enum SecretSource {
    Classical,
    HybridPQ,
    Derived,
}
```

#### KeyExchange Trait (B)

```rust
// v1: Single method
pub trait KeyExchange {
    fn exchange(&self, peer_public: &[u8]) -> Result<SharedSecret>;
}

// v2: Expanded trait with async support
pub trait KeyExchange: Send + Sync {
    type PublicKey: AsRef<[u8]>;
    type Ciphertext: AsRef<[u8]>;

    fn public_key(&self) -> Self::PublicKey;

    // KEM-style interface
    fn encapsulate(&self, peer_public: &Self::PublicKey)
        -> Result<(SharedSecret, Self::Ciphertext)>;

    fn decapsulate(&self, ciphertext: &Self::Ciphertext)
        -> Result<SharedSecret>;

    // Async variants
    async fn encapsulate_async(&self, peer_public: &Self::PublicKey)
        -> Result<(SharedSecret, Self::Ciphertext)> {
        self.encapsulate(peer_public)
    }
}
```

### Transport Types

#### Transport Trait (B)

```rust
// v1: Simple sync/async split
pub trait Transport {
    fn send(&self, data: &[u8]) -> io::Result<()>;
    fn recv(&self, buf: &mut [u8]) -> io::Result<usize>;
}

pub trait AsyncTransport {
    async fn send(&self, data: &[u8]) -> io::Result<()>;
    async fn recv(&self, buf: &mut [u8]) -> io::Result<usize>;
}

// v2: Unified async trait with capabilities
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Send data
    async fn send(&self, data: &[u8]) -> Result<()>;

    /// Receive data
    async fn recv(&self, buf: &mut [u8]) -> Result<usize>;

    /// Transport type identifier
    fn transport_type(&self) -> TransportType;

    /// Maximum transmission unit
    fn mtu(&self) -> usize;

    /// Whether transport supports connection migration
    fn supports_migration(&self) -> bool;

    /// Current latency estimate
    fn latency_estimate(&self) -> Duration;

    /// Transport statistics
    fn stats(&self) -> TransportStats;

    /// Close transport
    async fn close(&self) -> Result<()>;
}
```

---

## New APIs

### Hybrid Cryptography

```rust
/// New hybrid key pair for post-quantum security
pub struct HybridKeyPair {
    pub classical: ClassicalKeyPair,
    pub post_quantum: PQKeyPair,
}

impl HybridKeyPair {
    /// Generate new hybrid key pair
    pub fn generate() -> Self;

    /// Generate from seed (deterministic)
    pub fn from_seed(seed: &[u8; 64]) -> Self;

    /// Public key for sharing
    pub fn public_key(&self) -> HybridPublicKey;

    /// Encapsulate to peer
    pub fn encapsulate(&self, peer: &HybridPublicKey)
        -> Result<(SharedSecret, HybridCiphertext)>;

    /// Decapsulate from ciphertext
    pub fn decapsulate(&self, ciphertext: &HybridCiphertext)
        -> Result<SharedSecret>;
}

/// Hybrid public key
pub struct HybridPublicKey {
    pub classical: ClassicalPublicKey,
    pub post_quantum: PQPublicKey,
}

/// Hybrid ciphertext
pub struct HybridCiphertext {
    pub classical: ClassicalCiphertext,
    pub post_quantum: PQCiphertext,
}
```

### Polymorphic Wire Format

```rust
/// Session-derived wire format for traffic analysis resistance
pub struct PolymorphicFormat {
    format_key: FormatKey,
    field_positions: FieldPositions,
    xor_mask: [u8; 32],
}

impl PolymorphicFormat {
    /// Derive format from session secret
    pub fn derive(session_secret: &[u8; 32]) -> Self;

    /// Encode frame header
    pub fn encode_header(&self, header: &FrameHeader) -> [u8; 24];

    /// Decode frame header
    pub fn decode_header(&self, data: &[u8; 24]) -> Result<FrameHeader>;

    /// Get header size
    pub fn header_size(&self) -> usize;
}
```

### Multi-Stream Support

```rust
/// Stream within a session
pub struct Stream {
    id: StreamId,
    role: StreamRole,
    state: StreamState,
    priority: Priority,
    window: FlowWindow,
}

impl Stream {
    /// Open new stream on session
    pub async fn open(session: &Session, priority: Priority) -> Result<Self>;

    /// Send data on stream
    pub async fn send(&self, data: &[u8]) -> Result<()>;

    /// Receive data from stream
    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize>;

    /// Get stream ID
    pub fn id(&self) -> StreamId;

    /// Close stream gracefully
    pub async fn close(&self) -> Result<()>;

    /// Reset stream immediately
    pub async fn reset(&self, code: ErrorCode) -> Result<()>;
}

/// Stream priority levels
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Background = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Urgent = 4,
}
```

### Per-Packet Ratchet

```rust
/// Per-packet forward secrecy ratchet
pub struct PacketRatchet {
    chain_key: ChainKey,
    packet_number: u64,
    key_cache: LruCache<u64, MessageKey>,
}

impl PacketRatchet {
    /// Create from initial secret
    pub fn new(initial_secret: &[u8; 32]) -> Self;

    /// Get key for next outgoing packet
    pub fn next_send_key(&mut self) -> (u64, MessageKey);

    /// Get key for received packet (may be out of order)
    pub fn key_for_packet(&mut self, packet_number: u64) -> Result<MessageKey>;

    /// Current packet number
    pub fn current_packet_number(&self) -> u64;
}
```

### Transport Manager

```rust
/// Manage multiple transports with fallback
pub struct TransportManager {
    transports: Vec<Arc<dyn Transport>>,
    primary: AtomicUsize,
    selector: TransportSelector,
}

impl TransportManager {
    /// Create with transports
    pub fn new(transports: Vec<Arc<dyn Transport>>) -> Self;

    /// Send using best available transport
    pub async fn send(&self, data: &[u8]) -> Result<()>;

    /// Migrate to different transport
    pub async fn migrate(&self, target: TransportType) -> Result<()>;

    /// Add new transport
    pub fn add_transport(&mut self, transport: Arc<dyn Transport>);

    /// Get primary transport
    pub fn primary(&self) -> Arc<dyn Transport>;

    /// Get all transports
    pub fn all_transports(&self) -> &[Arc<dyn Transport>];
}
```

### Group Communication (v2.1)

```rust
/// Group communication using TreeKEM
pub struct Group {
    id: GroupId,
    epoch: u64,
    tree: RatchetTree,
    members: Vec<GroupMember>,
}

impl Group {
    /// Create new group
    pub fn create(creator: &Identity) -> Self;

    /// Add member to group
    pub async fn add_member(&mut self, member: &Identity) -> Result<GroupWelcome>;

    /// Remove member from group
    pub async fn remove_member(&mut self, member_id: &MemberId) -> Result<()>;

    /// Send message to group
    pub async fn send(&self, message: &[u8]) -> Result<()>;

    /// Update own key material
    pub async fn update(&mut self) -> Result<()>;

    /// Process received group message
    pub async fn process_message(&mut self, message: &GroupMessage)
        -> Result<Vec<u8>>;
}
```

---

## Deprecated APIs

### v1 Compatibility APIs (D)

```rust
// These APIs are deprecated and will be removed in v3

/// DEPRECATED: Use Session::builder() instead
#[deprecated(since = "2.0.0", note = "Use Session::builder() instead")]
impl Session {
    pub fn new(
        connection_id: ConnectionId,
        peer_addr: SocketAddr,
        shared_secret: [u8; 32],
    ) -> Result<Self> {
        Self::builder()
            .connection_id(connection_id)
            .peer_addr(peer_addr)
            .crypto_context(CryptoContext::from_classical(shared_secret))
            .build()
    }
}

/// DEPRECATED: Use async send() instead
#[deprecated(since = "2.0.0", note = "Use async send() instead")]
impl Session {
    pub fn send_sync(&self, data: &[u8]) -> Result<()> {
        self.send_blocking(data)
    }
}

/// DEPRECATED: Use HybridKeyPair instead
#[deprecated(since = "2.0.0", note = "Use HybridKeyPair instead")]
pub type X25519KeyPair = ClassicalKeyPair;

/// DEPRECATED: Fixed padding classes
#[deprecated(since = "2.0.0", note = "Use ContinuousPadding instead")]
pub enum FixedPaddingClass {
    Class64,
    Class128,
    Class256,
    // ...
}
```

### Removed APIs (R)

```rust
// These APIs have been removed in v2

// REMOVED: sync-only transport
// pub trait Transport { fn send(&self, data: &[u8]) -> io::Result<()>; }

// REMOVED: HKDF-SHA256 (replaced with HKDF-BLAKE3)
// pub fn hkdf_sha256(secret: &[u8], info: &[u8], len: usize) -> Vec<u8>;

// REMOVED: Per-minute ratchet
// pub struct MinuteRatchet { ... }

// REMOVED: Fixed padding
// pub fn apply_fixed_padding(data: &[u8], class: PaddingClass) -> Vec<u8>;
```

---

## Crate-by-Crate Changes

### wraith-core

| Change | Type | Description |
|--------|------|-------------|
| `ConnectionId` expansion | B | 64-bit to 128-bit |
| `FrameHeader` fields | B | Expanded sequence, length |
| `Session::new()` | D | Use `Session::builder()` |
| `Session::builder()` | A | New builder API |
| `Stream` | A | Multi-stream support |
| `StreamId` | A | Stream identifier type |
| `Priority` | A | Stream priority enum |
| `SessionBuilder` | A | Builder pattern |
| `WireFormat` | A | Polymorphic format |
| `FrameType` expansion | M | More frame types |

### wraith-crypto

| Change | Type | Description |
|--------|------|-------------|
| `HybridKeyPair` | A | Post-quantum hybrid |
| `HybridPublicKey` | A | Hybrid public key |
| `HybridCiphertext` | A | Hybrid ciphertext |
| `PacketRatchet` | A | Per-packet FS |
| `MlKem768` | A | Post-quantum KEM |
| `MlDsa65` | A | Post-quantum signatures |
| `hkdf_sha256` | R | Removed, use BLAKE3 |
| `hkdf_blake3` | A | New KDF |
| `SharedSecret::from_hybrid` | A | Hybrid secret creation |
| `KeyExchange` trait | B | KEM-style interface |

### wraith-transport

| Change | Type | Description |
|--------|------|-------------|
| `Transport` trait | B | Unified async trait |
| `AsyncTransport` | R | Merged into Transport |
| `TransportManager` | A | Multi-transport |
| `TransportType` | A | Transport enum |
| `QuicTransport` | A | QUIC support |
| `WebSocketTransport` | A | WebSocket support |
| `Http2Transport` | A | HTTP/2 support |
| `AfXdpTransport` | A | Full AF_XDP impl |
| `TransportStats` | A | Statistics struct |
| `TransportSelector` | A | Selection strategy |

### wraith-obfuscation

| Change | Type | Description |
|--------|------|-------------|
| `PolymorphicFormat` | A | Session-derived format |
| `ContinuousPadding` | A | Continuous distribution |
| `FixedPaddingClass` | D | Use continuous |
| `TimingJitter` | M | Enhanced Markov model |
| `ProtocolMimicry` | M | New profiles |
| `FormatKey` | A | Format derivation key |

### wraith-discovery

| Change | Type | Description |
|--------|------|-------------|
| `PeerInfo` | M | Extended fields |
| `DhtNode` | M | v2 protocol support |
| `IceAgent` | A | Full ICE support |
| `RelayClient` | M | Enhanced relay |
| `StunClient` | M | Extended attributes |

### wraith-files

| Change | Type | Description |
|--------|------|-------------|
| `FileTransfer` | M | Multi-stream support |
| `ChunkHash` | M | BLAKE3 tree |
| `DeltaTransfer` | A | rsync-style |
| `ContentAddress` | A | CAS support |

---

## Migration Examples

### Example 1: Session Creation

```rust
// ═══════════════════════════════════════════════════
// v1 Code
// ═══════════════════════════════════════════════════
use wraith_core::{Session, ConnectionId};

fn create_session_v1(
    peer_addr: SocketAddr,
    shared_secret: [u8; 32],
) -> Result<Session> {
    let cid = ConnectionId::new();
    Session::new(cid, peer_addr, shared_secret)
}

fn send_data_v1(session: &Session, data: &[u8]) -> Result<()> {
    session.send(data)  // Sync
}

// ═══════════════════════════════════════════════════
// v2 Code
// ═══════════════════════════════════════════════════
use wraith_core::{Session, ConnectionId, CryptoContext};

async fn create_session_v2(
    peer_addr: SocketAddr,
    shared_secret: [u8; 32],
) -> Result<Session> {
    let cid = ConnectionId::generate();
    Session::builder()
        .connection_id(cid)
        .peer_addr(peer_addr)
        .crypto_context(CryptoContext::from_classical(shared_secret))
        .build()
}

async fn send_data_v2(session: &Session, data: &[u8]) -> Result<()> {
    session.send(data).await  // Async
}
```

### Example 2: Key Exchange

```rust
// ═══════════════════════════════════════════════════
// v1 Code
// ═══════════════════════════════════════════════════
use wraith_crypto::{X25519KeyPair, SharedSecret};

fn key_exchange_v1(peer_public: &[u8; 32]) -> SharedSecret {
    let keypair = X25519KeyPair::generate();
    let public = keypair.public_key();

    // Send public to peer...

    keypair.diffie_hellman(peer_public)
}

// ═══════════════════════════════════════════════════
// v2 Code
// ═══════════════════════════════════════════════════
use wraith_crypto::{HybridKeyPair, HybridPublicKey, SharedSecret};

fn key_exchange_v2(peer_public: &HybridPublicKey) -> Result<SharedSecret> {
    let keypair = HybridKeyPair::generate();
    let public = keypair.public_key();

    // Send public to peer...

    let (shared_secret, ciphertext) = keypair.encapsulate(peer_public)?;

    // Send ciphertext to peer...

    Ok(shared_secret)
}
```

### Example 3: Transport Usage

```rust
// ═══════════════════════════════════════════════════
// v1 Code
// ═══════════════════════════════════════════════════
use wraith_transport::{UdpTransport, Transport};

fn send_packet_v1(transport: &UdpTransport, data: &[u8]) -> io::Result<()> {
    transport.send(data)  // Sync
}

// ═══════════════════════════════════════════════════
// v2 Code
// ═══════════════════════════════════════════════════
use wraith_transport::{TransportManager, Transport};

async fn send_packet_v2(
    manager: &TransportManager,
    data: &[u8],
) -> Result<()> {
    manager.send(data).await  // Async, with fallback
}

// Or with specific transport
async fn send_on_quic_v2(
    manager: &TransportManager,
    data: &[u8],
) -> Result<()> {
    manager.migrate(TransportType::Quic).await?;
    manager.send(data).await
}
```

---

## Compatibility Shims

### Automatic Migration Shim

```rust
/// Compatibility shim for v1 code
#[cfg(feature = "v1-compat")]
pub mod v1_compat {
    use super::*;

    /// v1-style ConnectionId
    pub type ConnectionIdV1 = u64;

    /// Convert v1 CID to v2
    pub fn cid_to_v2(v1_cid: ConnectionIdV1) -> ConnectionId {
        ConnectionId::from_v1(v1_cid)
    }

    /// v1-style Session::new
    pub fn session_new(
        cid: ConnectionIdV1,
        addr: SocketAddr,
        secret: [u8; 32],
    ) -> Result<Session> {
        Session::builder()
            .connection_id(ConnectionId::from_v1(cid))
            .peer_addr(addr)
            .crypto_context(CryptoContext::from_classical(secret))
            .v1_compat(true)
            .build()
    }

    /// v1-style sync send (blocks)
    pub fn session_send_sync(session: &Session, data: &[u8]) -> Result<()> {
        session.send_blocking(data)
    }
}
```

### Feature Flag Configuration

```toml
# Cargo.toml
[features]
default = ["v2-native"]

# Full v2 functionality
v2-native = []

# Enable v1 compatibility APIs
v1-compat = []

# Classical crypto only (no PQ)
classical-only = []

# Deprecated API warnings as errors
strict = []
```

---

## Related Documents

- [Migration Guide](09-WRAITH-Protocol-v2-Migration-Guide.md) - Step-by-step migration
- [Compatibility Matrix](15-WRAITH-Protocol-v2-Compatibility-Matrix.md) - Version compatibility
- [Specification](01-WRAITH-Protocol-v2-Specification.md) - Protocol specification
- [API Reference](08-WRAITH-Protocol-v2-API-Reference.md) - Full API docs

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial API changes document |
