# WRAITH Protocol v1 to v2 Changelog

**Document Version:** 2.0.0  
**Status:** Change Documentation  
**Date:** January 2026  

---

## Executive Summary

WRAITH Protocol v2 represents a major evolution from v1, introducing post-quantum cryptography, enhanced traffic analysis resistance, cross-platform support, and native group communication. This document provides a comprehensive changelog of all modifications, additions, and deprecations.

---

## Table of Contents

1. [Version Overview](#1-version-overview)
2. [Breaking Changes](#2-breaking-changes)
3. [Cryptographic Changes](#3-cryptographic-changes)
4. [Wire Protocol Changes](#4-wire-protocol-changes)
5. [Obfuscation Changes](#5-obfuscation-changes)
6. [Transport Layer Changes](#6-transport-layer-changes)
7. [Session Management Changes](#7-session-management-changes)
8. [New Features](#8-new-features)
9. [Deprecated Features](#9-deprecated-features)
10. [Migration Notes](#10-migration-notes)

---

## 1. Version Overview

### 1.1 Version Comparison Matrix

| Aspect | v1.0.0-DRAFT | v2.0.0 | Change Type |
|--------|--------------|--------|-------------|
| Protocol Version | `0x01` | `0x02` | Breaking |
| Key Exchange | X25519 | X25519 + ML-KEM-768 | Enhanced |
| Wire Format | Fixed | Polymorphic | Breaking |
| Transports | UDP only | Pluggable (7+ options) | Enhanced |
| Platforms | Linux | Cross-platform + WASM | Enhanced |
| Groups | None | Native TreeKEM | New |
| Real-time | None | QoS + FEC | New |
| Probing Resistance | None | Full | New |

### 1.2 Compatibility

**v2 is NOT backward compatible with v1.**

- v2 clients cannot connect to v1 servers
- v1 clients cannot connect to v2 servers
- Migration requires coordinated upgrade of all endpoints

### 1.3 Version Negotiation

v2 introduces version negotiation in the handshake:

```
v1: No version field (implicit v1)
v2: Version byte in proof packet header

v2 Proof Packet:
┌────────────────────────────────────────────────────────────────┐
│ Version (1B) │ Timestamp (8B) │ Random (16B) │ Proof (32B) │ ...
└────────────────────────────────────────────────────────────────┘
```

---

## 2. Breaking Changes

### 2.1 Wire Format

**v1 Wire Format (Fixed):**
```
┌────────────────────────────────────────────────────────────────┐
│ CID (8B) │ Encrypted Payload (variable) │ Auth Tag (16B)      │
└────────────────────────────────────────────────────────────────┘
```

**v2 Wire Format (Polymorphic):**
```
┌────────────────────────────────────────────────────────────────┐
│ [Session-derived field order and sizes]                        │
│ Fields: CID, Payload, Tag, optional Dummy, Length, Version     │
└────────────────────────────────────────────────────────────────┘

Format derived from: HKDF(session_secret, "wire-format-v2", 32)
```

**Impact:** Complete wire format incompatibility. Packets from v1 will fail to parse in v2 and vice versa.

### 2.2 Handshake Protocol

**v1 Handshake:**
```
Phase 1: e
Phase 2: e, ee, s, es
Phase 3: s, se
```

**v2 Handshake:**
```
Phase 1: proof, e, [pq_pk]
Phase 2: e, ee, [pq_ct], s, es
Phase 3: s, se, extensions
```

**Changes:**
| Change | Description |
|--------|-------------|
| `proof` added | Probing resistance - proof of server knowledge |
| `pq_pk` added | Post-quantum public key (optional) |
| `pq_ct` added | Post-quantum ciphertext (optional) |
| `extensions` added | Feature negotiation |

### 2.3 Frame Header

**v1 Inner Frame Header (20 bytes):**
```
┌────────────────────────────────────────────────────────────────┐
│ Nonce (8B) │ Type (1B) │ Flags (1B) │ StreamID (2B) │         │
│ Seq (4B) │ PayloadLen (2B) │ Reserved (2B)                    │
└────────────────────────────────────────────────────────────────┘
```

**v2 Inner Frame Header (24 bytes):**
```
┌────────────────────────────────────────────────────────────────┐
│ Nonce (8B) │ Type (1B) │ Flags (1B) │ StreamID (2B) │ Seq (4B)│
│ Offset (8B) │ PayloadLen (2B) │ ExtCount (1B) │ [Extensions]  │
└────────────────────────────────────────────────────────────────┘
```

**Changes:**
| Field | v1 | v2 | Notes |
|-------|----|----|-------|
| Offset | Not present | 8 bytes | Supports >4GB transfers |
| ExtCount | Not present | 1 byte | Extension framework |
| Extensions | Not present | Variable | Per-frame extensions |
| Reserved | 2 bytes | Removed | Replaced by useful fields |

### 2.4 Frame Types

**New Frame Types in v2:**

| Value | Type | Description |
|-------|------|-------------|
| `0x10` | GROUP_JOIN | Join group session |
| `0x11` | GROUP_LEAVE | Leave group |
| `0x12` | GROUP_REKEY | Group key update |
| `0x13` | QOS_UPDATE | QoS parameter change |
| `0x14` | FEC_REPAIR | Forward error correction |
| `0x15` | PRIORITY | Stream priority update |
| `0x16` | DATAGRAM | Unreliable datagram |
| `0x17` | TIMESTAMP | High-precision timing |

**Modified Frame Types:**

| Type | v1 Behavior | v2 Behavior |
|------|-------------|-------------|
| REKEY | X25519 only | X25519 + optional ML-KEM |
| PAD | Fixed sizes | Continuous distribution |

### 2.5 Key Derivation

**v1 Key Derivation:**
```
PRK = HKDF-Extract(salt="wraith-v1", IKM=DH_outputs)
send_key = HKDF-Expand(PRK, "send", 32)
recv_key = HKDF-Expand(PRK, "recv", 32)
```

**v2 Key Derivation:**
```
PRK = HKDF-Extract(salt="wraith-v2-" || version, IKM=DH_outputs || PQ_SS)

initiator_send_key = HKDF-Expand(PRK, "i2r-data" || session_id, 32)
responder_send_key = HKDF-Expand(PRK, "r2i-data" || session_id, 32)
wire_format_seed = HKDF-Expand(PRK, "wire-format", 32)
padding_seed = HKDF-Expand(PRK, "padding", 32)
// ... additional derived keys
```

**Changes:**
- Directional key naming (clearer semantics)
- Session ID binding (prevents key confusion)
- Additional derived seeds (wire format, padding)
- Optional PQ shared secret in IKM

---

## 3. Cryptographic Changes

### 3.1 Key Exchange

**v1:**
- X25519 only
- 128-bit classical security
- Vulnerable to quantum computers

**v2:**
- Hybrid: X25519 + ML-KEM-768
- 128-bit classical + 128-bit post-quantum security
- Secure if EITHER algorithm is secure

```rust
// v1 Key Exchange
let shared_secret = x25519(my_secret, peer_public);

// v2 Hybrid Key Exchange
let classical_ss = x25519(my_secret, peer_public);
let (pq_ct, pq_ss) = ml_kem_768::encapsulate(peer_pq_public);
let combined_ss = blake3::derive_key(
    "wraith-hybrid-kem-v2",
    &[classical_ss, pq_ss].concat()
);
```

### 3.2 Ratcheting

**v1 DH Ratchet:**
- Triggered every 60 seconds OR 100,000 packets
- X25519 only

**v2 DH Ratchet:**
- Triggered every 120 seconds OR 1,000,000 packets
- X25519 + optional ML-KEM refresh
- Configurable triggers

```rust
// v2 Ratchet configuration
pub struct RatchetConfig {
    /// Time-based trigger (default: 120s)
    pub time_interval: Duration,
    
    /// Packet-based trigger (default: 1M)
    pub packet_interval: u64,
    
    /// Include PQ ratchet (default: true if PQ enabled)
    pub include_pq: bool,
}
```

### 3.3 Algorithm Agility

**v1:** Fixed algorithm suite, no negotiation

**v2:** Negotiated algorithm suites

```rust
// v2 Suite negotiation
pub const ALLOWED_SUITES: &[CryptoSuite] = &[
    // Suite A: Default
    CryptoSuite {
        aead: XChaCha20Poly1305,
        hash: Blake3,
        kex: X25519_MlKem768,
        sig: Ed25519,
    },
    // Suite B: Hardware acceleration
    CryptoSuite {
        aead: Aes256Gcm,
        hash: Sha256,
        kex: X25519_MlKem768,
        sig: Ed25519,
    },
    // Suite C: Maximum security
    CryptoSuite {
        aead: XChaCha20Poly1305,
        hash: Blake3,
        kex: X448_MlKem1024,
        sig: Ed448,
    },
];
```

---

## 4. Wire Protocol Changes

### 4.1 Connection ID

**v1:**
- Fixed 8-byte CID
- Random generation
- No rotation during session

**v2:**
- Variable CID size (4-8 bytes, negotiated)
- Elligator2-compatible generation
- Periodic rotation with PATH_CHALLENGE

```rust
// v2 CID rotation
impl Session {
    pub fn rotate_cid(&mut self) -> ConnectionId {
        let new_cid = self.derive_new_cid();
        self.send_path_challenge(new_cid);
        new_cid
    }
}
```

### 4.2 Packet Size

**v1:**
- Fixed padding classes: 64, 256, 512, 1024, 1472, 8960 bytes
- Easily fingerprintable distribution

**v2:**
- Continuous padding distribution
- Configurable profiles (Uniform, HTTPS Empirical, Gaussian, Custom)
- No fixed size classes

```rust
// v1 Padding (fingerptintable)
fn pad_v1(payload_len: usize) -> usize {
    match payload_len {
        0..=64 => 64,
        65..=256 => 256,
        257..=512 => 512,
        513..=1024 => 1024,
        1025..=1472 => 1472,
        _ => 8960,
    }
}

// v2 Padding (continuous distribution)
fn pad_v2(payload_len: usize, config: &PaddingConfig) -> usize {
    let min_size = payload_len + OVERHEAD;
    config.distribution.sample(min_size)
}
```

### 4.3 Flow Control

**v1:**
- Simple window-based flow control
- Fixed initial window (1 MB)
- No connection-level flow control

**v2:**
- Hierarchical flow control (connection + stream)
- Configurable initial window
- WINDOW_UPDATE frames for both levels

```rust
// v2 Flow control
pub struct FlowControl {
    /// Connection-level window
    pub connection_window: u64,
    
    /// Per-stream windows
    pub stream_windows: HashMap<StreamId, u64>,
    
    /// Configurable limits
    pub config: FlowControlConfig,
}

pub struct FlowControlConfig {
    pub initial_connection_window: u64,  // Default: 16 MB
    pub initial_stream_window: u64,      // Default: 1 MB
    pub max_connection_window: u64,      // Default: 64 MB
    pub max_stream_window: u64,          // Default: 16 MB
}
```

---

## 5. Obfuscation Changes

### 5.1 Elligator2 Encoding

**v1:**
- Optional Elligator2 encoding
- Not always enabled

**v2:**
- Mandatory Elligator2 for all public keys in handshake
- High bit randomization for additional entropy

```rust
// v2 Elligator2 (mandatory)
pub fn generate_handshake_keypair() -> (SecretKey, Representative) {
    loop {
        let sk = SecretKey::random(&mut OsRng);
        let pk = PublicKey::from(&sk);
        
        if let Some(mut repr) = elligator2::encode(&pk) {
            // Randomize high bit
            repr[31] |= (OsRng.gen::<u8>() & 0x80);
            return (sk, repr);
        }
    }
}
```

### 5.2 Timing Obfuscation

**v1:**
- No timing obfuscation
- Packets sent immediately when ready

**v2:**
- Configurable timing modes
- Distribution matching (HTTPS, video, custom)
- HMM-based timing models

```rust
// v2 Timing modes
pub enum TimingMode {
    Disabled,                    // v1 behavior
    ConstantRate { pps: f64 },
    HttpsBrowsing { params },
    VideoStreaming { params },
    CustomHmm { model },
}
```

### 5.3 Cover Traffic

**v1:**
- Basic cover traffic (fixed interval random packets)

**v2:**
- Decoy streams (multiple fake transfers)
- Configurable mixing strategies
- Content-aware decoy generation

```rust
// v2 Decoy traffic
pub struct DecoyConfig {
    pub enabled: bool,
    pub stream_count: usize,          // Number of fake streams
    pub bandwidth: Bandwidth,          // Bandwidth for decoys
    pub content: DecoyContentGenerator,
    pub mixing: MixingStrategy,        // Replace, Additive, Interleave
}
```

### 5.4 Probing Resistance (NEW)

**v1:** No probing resistance

**v2:** Full probing resistance

```rust
// v2 Probing resistance
pub struct ProbingResistance {
    /// Require proof-of-knowledge in first packet
    pub require_proof: bool,
    
    /// Response to invalid probes
    pub probe_response: ProbeResponse,
    
    /// Optional service fronting
    pub fronting: Option<FrontingConfig>,
}

pub enum ProbeResponse {
    SilentDrop,
    MimicTls { certificate },
    MimicHttp { server_header, index_page },
    ProxyToBackend { backend, marker },
}
```

### 5.5 Entropy Normalization (NEW)

**v1:** Raw ciphertext (~8 bits/byte entropy)

**v2:** Configurable entropy normalization

```rust
// v2 Entropy normalization
pub enum EntropyNormalization {
    None,                              // v1 behavior
    PredictableInsertion { ratio },    // Insert known bytes
    Base64,                            // ~6 bits/byte
    JsonWrapper { template },          // Looks like API traffic
    HttpChunked,                       // HTTP-like framing
}
```

---

## 6. Transport Layer Changes

### 6.1 Transport Abstraction

**v1:** UDP only, hardcoded

**v2:** Pluggable transport abstraction

```rust
// v2 Transport trait
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, packet: &[u8]) -> Result<()>;
    async fn recv(&self, buf: &mut [u8]) -> Result<usize>;
    fn characteristics(&self) -> TransportCharacteristics;
    fn mtu(&self) -> usize;
}

// Available transports
pub enum TransportBinding {
    Udp(UdpTransport),
    Tcp(TcpTransport),
    WebSocket(WebSocketTransport),
    Http2(Http2Transport),
    Quic(QuicTransport),
    Icmp(IcmpTransport),
    Dns(DnsTransport),
    RawEthernet(AfXdpTransport),
}
```

### 6.2 Kernel Bypass

**v1:** Standard sockets only

**v2:** Optional kernel bypass (AF_XDP + io_uring)

```rust
// v2 Kernel bypass (Linux only)
#[cfg(target_os = "linux")]
pub struct AfXdpTransport {
    socket: XskSocket,
    umem: Umem,
    // Zero-copy packet processing
}

// Performance comparison
// Standard sockets: 1-10 Gbps
// io_uring:         5-20 Gbps  
// AF_XDP:           40-100 Gbps
```

### 6.3 Platform Support

**v1:** Linux native only

**v2:** Cross-platform

| Platform | v1 | v2 |
|----------|----|----|
| Linux | ✓ | ✓ (enhanced) |
| Windows | ✗ | ✓ |
| macOS | ✗ | ✓ |
| WASM/Browser | ✗ | ✓ |
| Embedded | ✗ | ✓ (no_std) |

---

## 7. Session Management Changes

### 7.1 Session States

**v1 States:**
```
CLOSED → CONNECTING → ESTABLISHED → CLOSED
```

**v2 States:**
```
CLOSED → CONNECTING → ESTABLISHED → CLOSED
                           │
                           ├── REKEYING → ESTABLISHED
                           ├── MIGRATING → ESTABLISHED
                           ├── RESUMING → ESTABLISHED
                           └── DRAINING → CLOSED
```

### 7.2 Session Resumption (NEW)

**v1:** No session resumption

**v2:** Ticket-based resumption

```rust
// v2 Session resumption
pub struct ResumptionTicket {
    pub id: [u8; 16],
    pub session_id: SessionId,
    pub resumption_secret: [u8; 32],
    pub server_fingerprint: [u8; 32],
    pub expires: SystemTime,
    pub encrypted_params: Vec<u8>,
}

// Resumption handshake: 0.5 RTT instead of 1.5 RTT
```

### 7.3 Connection Migration (Enhanced)

**v1:**
- Basic PATH_CHALLENGE/PATH_RESPONSE
- Single path only

**v2:**
- Enhanced path validation
- Multi-path support (extension)
- Seamless NAT rebinding

---

## 8. New Features

### 8.1 Group Communication

**Added in v2:**
- Native multi-party sessions
- TreeKEM group key management
- O(log n) key update messages
- Forward secrecy for groups

```rust
// v2 Group session
pub struct GroupSession {
    pub group_id: GroupId,
    pub group_key: GroupKey,
    pub members: HashMap<PeerId, MemberInfo>,
    pub topology: GroupTopology,  // FullMesh, Tree, Gossip
    pub tree_kem: TreeKem,
}
```

### 8.2 Real-Time Extensions

**Added in v2:**
- QoS modes (Reliable, UnreliableOrdered, UnreliableUnordered, PartiallyReliable)
- Forward Error Correction (XOR, Reed-Solomon, LDPC)
- Priority streams
- Jitter buffer support

```rust
// v2 QoS configuration
pub struct QosConfig {
    pub mode: QosMode,
    pub target_latency: Duration,
    pub jitter_buffer: Duration,
    pub fec: Option<FecConfig>,
}
```

### 8.3 Content-Addressed Storage

**Added in v2:**
- Merkle tree file representation
- Content-defined chunking
- Deduplication support
- Resumable transfers

```rust
// v2 Content addressing
pub struct MerkleFile {
    pub root: Hash,
    pub size: u64,
    pub tree: MerkleTree,
}

impl MerkleFile {
    pub fn chunks_for_range(&self, start: u64, end: u64) -> Vec<ChunkId>;
    pub fn verify_chunk(&self, id: &ChunkId, data: &[u8]) -> bool;
}
```

### 8.4 Resource Profiles

**Added in v2:**
- Performance profile (kernel bypass, large buffers)
- Balanced profile (standard settings)
- Constrained profile (mobile/IoT)
- Stealth profile (maximum obfuscation)
- Metered profile (bandwidth budget)

### 8.5 Extension Framework

**Added in v2:**
- Negotiated extensions
- Custom extension API
- Built-in extensions: POST_QUANTUM, REAL_TIME_QOS, GROUP_V2, etc.

---

## 9. Deprecated Features

### 9.1 Removed in v2

| Feature | v1 | v2 | Reason |
|---------|----|----|--------|
| Fixed padding classes | ✓ | ✗ | Fingerprintable |
| UDP-only transport | ✓ | ✗ | Too restrictive |
| Single platform | ✓ | ✗ | Limited deployment |
| Fixed wire format | ✓ | ✗ | Fingerprintable |

### 9.2 Changed Defaults

| Setting | v1 Default | v2 Default | Reason |
|---------|------------|------------|--------|
| Rekey interval | 60s | 120s | Balance security/overhead |
| Rekey packets | 100K | 1M | Modern hardware handles more |
| Initial window | 1 MB | 16 MB | Higher bandwidth paths |
| Chunk size | 64 KB | 256 KB | Better throughput |

---

## 10. Migration Notes

### 10.1 Coordinated Upgrade

Migration from v1 to v2 requires coordinated upgrade:

```
Phase 1: Deploy v2 servers alongside v1
Phase 2: Upgrade clients to v2
Phase 3: Migrate traffic to v2 servers
Phase 4: Decommission v1 servers
```

### 10.2 Configuration Migration

**v1 Configuration:**
```toml
[crypto]
ratchet_interval_secs = 60

[transport]
# UDP only, no configuration needed

[obfuscation]
padding_mode = "classes"
cover_traffic = true
```

**v2 Configuration:**
```toml
[crypto]
suite = "default"  # or "hardware", "maximum"
post_quantum = true
ratchet_time_interval_secs = 120
ratchet_packet_interval = 1000000

[transport]
type = "auto"  # or "udp", "tcp", "websocket", "http2", "quic"
kernel_bypass = false  # Linux only

[obfuscation]
padding_distribution = "https_empirical"
timing_mode = "https_browsing"
cover_traffic = true
probing_resistance = true
entropy_normalization = "none"

[profile]
type = "balanced"  # or "performance", "constrained", "stealth", "metered"
```

### 10.3 API Changes

**v1 API:**
```rust
// v1 Session creation
let session = Session::connect(peer_addr, static_key)?;
session.send_file(path)?;
```

**v2 API:**
```rust
// v2 Session creation (with configuration)
let config = SessionConfig::balanced()
    .with_transport(TransportType::Auto)
    .with_post_quantum(true)
    .with_obfuscation(ObfuscationConfig::stealth());

let session = Session::connect(peer_addr, static_key, config).await?;

// v2 File transfer (with options)
let transfer = session.send_file(path, TransferOptions {
    qos: QosMode::Reliable,
    compression: true,
    resume: true,
}).await?;
```

### 10.4 Key Material

**v1 keys are compatible with v2** for the classical component:

- Static Ed25519 identity keys: Compatible
- Static X25519 keys: Compatible
- Ephemeral keys: Generated per-session (no migration needed)

**New in v2:**
- ML-KEM-768 keys: Must be generated (optional)

### 10.5 Interoperability Period

If gradual migration is needed:

1. **Gateway approach:** Deploy v1↔v2 gateway that translates between versions
2. **Dual-stack:** Clients support both v1 and v2, negotiate on connection
3. **Feature flags:** v2 with v1-compatible mode (reduced features)

---

## Change Summary by Component

| Component | Breaking | Enhanced | New |
|-----------|----------|----------|-----|
| Wire Format | ● | | |
| Handshake | ● | | |
| Frame Header | ● | | |
| Key Exchange | | ● | |
| Ratcheting | | ● | |
| Padding | ● | | |
| Transport | | ● | |
| Flow Control | | ● | |
| Session States | | ● | |
| Groups | | | ● |
| Real-Time | | | ● |
| Probing Resistance | | | ● |
| Resource Profiles | | | ● |

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2026-01 | Initial changelog |

---

*End of WRAITH Protocol v1 to v2 Changelog*
