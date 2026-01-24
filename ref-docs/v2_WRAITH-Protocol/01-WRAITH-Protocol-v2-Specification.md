# WRAITH Protocol v2 Technical Specification

**Document Version:** 2.0.0  
**Status:** Technical Specification  
**Classification:** Implementation Reference  
**Target Platform:** Cross-Platform (Linux Primary, Windows, macOS, WASM)  
**Primary Language:** Rust (2021 Edition)  
**Date:** January 2026  

---

## Document Information

| Field | Value |
|-------|-------|
| **Protocol Name** | WRAITH (Wire-Resistant Authenticated Integrity Transport with Hardening) |
| **Version** | 2.0.0 |
| **Supersedes** | WRAITH Protocol v1.0.0-DRAFT |
| **Security Level** | Post-Quantum Hybrid (Classical + ML-KEM) |
| **Target Throughput** | 1-100 Gbps (platform dependent) |

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Design Philosophy](#2-design-philosophy)
3. [Protocol Architecture](#3-protocol-architecture)
4. [Wire Protocol Specification](#4-wire-protocol-specification)
5. [Cryptographic Protocol](#5-cryptographic-protocol)
6. [Traffic Obfuscation System](#6-traffic-obfuscation-system)
7. [Transport Abstraction Layer](#7-transport-abstraction-layer)
8. [Session Management](#8-session-management)
9. [Stream Multiplexing](#9-stream-multiplexing)
10. [Flow Control and Congestion](#10-flow-control-and-congestion)
11. [NAT Traversal](#11-nat-traversal)
12. [Discovery Protocol](#12-discovery-protocol)
13. [Group Communication](#13-group-communication)
14. [Real-Time Extensions](#14-real-time-extensions)
15. [Error Handling](#15-error-handling)
16. [Security Properties](#16-security-properties)
17. [Protocol Constants](#17-protocol-constants)
18. [Appendices](#18-appendices)

---

## 1. Executive Summary

### 1.1 Purpose

WRAITH Protocol v2 is a next-generation secure communication protocol designed for high-throughput, low-latency data transfer with strong cryptographic guarantees and advanced traffic analysis resistance. It provides:

- **Security**: Post-quantum hybrid cryptography with continuous forward secrecy
- **Performance**: Kernel-bypass capable, achieving 40-100 Gbps on modern hardware
- **Privacy**: Computational indistinguishability from legitimate traffic
- **Flexibility**: Runs over any transport (UDP, TCP, WebSocket, HTTP/2, QUIC)
- **Universality**: Native support for constrained devices to data centers

### 1.2 Key Improvements Over v1

| Aspect | v1 | v2 |
|--------|----|----|
| Key Exchange | X25519 only | Hybrid X25519 + ML-KEM-768 |
| Traffic Analysis | Fixed padding classes | Continuous distribution matching |
| Transport | UDP only | Pluggable (UDP, TCP, WS, HTTP/2, QUIC) |
| Platforms | Linux native | Cross-platform + WASM |
| Groups | Peer-to-peer only | Native multi-party support |
| Real-time | File transfer optimized | QoS modes for streaming |
| Detection Resistance | Basic obfuscation | Active probing resistance + polymorphism |

### 1.3 Threat Model

**Protected Against:**
- Passive network observers (ISPs, nation-states)
- Active network attackers (MITM, injection, replay)
- Traffic analysis (timing, size, pattern correlation)
- Active probing attacks (protocol identification)
- Quantum computers (via hybrid cryptography)
- Malicious DHT/relay nodes
- Connection tracking and fingerprinting

**Not Protected Against:**
- Endpoint compromise (malware on devices)
- Global passive adversary with traffic correlation across all paths
- Cryptographic breaks in underlying primitives
- Implementation-level side-channel attacks (mitigated, not eliminated)

---

## 2. Design Philosophy

### 2.1 Core Principles

1. **Defense in Depth**: Multiple independent security layers; compromise of one doesn't defeat others
2. **Fail Secure**: On error, default to most restrictive behavior
3. **Zero Trust**: Assume hostile network; authenticate and encrypt everything
4. **Minimal Fingerprint**: Protocol should be indistinguishable from noise or legitimate traffic
5. **Resource Aware**: Adapt to device capabilities and network conditions
6. **Future Proof**: Cryptographic agility with post-quantum readiness

### 2.2 Security vs. Performance Trade-offs

The protocol provides configurable profiles allowing users to choose their position on the security-performance spectrum:

```
┌─────────────────────────────────────────────────────────────────────────┐
│  STEALTH ◄──────────────────────────────────────────────► PERFORMANCE  │
│                                                                         │
│  • Full traffic shaping          • Minimal padding                     │
│  • Cover traffic enabled         • No cover traffic                    │
│  • Timing obfuscation            • Immediate transmission              │
│  • Protocol mimicry              • Raw UDP                             │
│  • Post-quantum keys             • Classical keys only                 │
│                                                                         │
│  Typical: 50-200 Mbps            Typical: 10-100 Gbps                  │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.3 Layered Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Application Layer                               │
│   File Transfer │ Streaming │ Messaging │ Custom Applications          │
├─────────────────────────────────────────────────────────────────────────┤
│                         Session Layer                                   │
│   Stream Mux │ Flow Control │ Congestion │ QoS │ Groups                │
├─────────────────────────────────────────────────────────────────────────┤
│                         Cryptographic Layer                             │
│   Noise_XX │ Hybrid KEM │ AEAD │ Ratcheting │ Authentication           │
├─────────────────────────────────────────────────────────────────────────┤
│                         Obfuscation Layer                               │
│   Elligator2 │ Traffic Shaping │ Timing │ Entropy │ Polymorphism       │
├─────────────────────────────────────────────────────────────────────────┤
│                         Transport Abstraction Layer                     │
│   UDP │ TCP │ WebSocket │ HTTP/2 │ QUIC │ Raw │ ICMP │ DNS             │
├─────────────────────────────────────────────────────────────────────────┤
│                         Platform Layer                                  │
│   AF_XDP │ io_uring │ Standard Sockets │ WASM │ Embedded               │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Protocol Architecture

### 3.1 Component Overview

```
                                    ┌─────────────────┐
                                    │   Application   │
                                    └────────┬────────┘
                                             │
                    ┌────────────────────────┼────────────────────────┐
                    │                        │                        │
              ┌─────▼─────┐            ┌─────▼─────┐            ┌─────▼─────┐
              │   File    │            │  Stream   │            │   Group   │
              │ Transfer  │            │   API     │            │   API     │
              └─────┬─────┘            └─────┬─────┘            └─────┬─────┘
                    │                        │                        │
                    └────────────────────────┼────────────────────────┘
                                             │
                                    ┌────────▼────────┐
                                    │  Session Mgmt   │
                                    │  ┌───────────┐  │
                                    │  │ Streams   │  │
                                    │  │ Flow Ctrl │  │
                                    │  │ Congestion│  │
                                    │  └───────────┘  │
                                    └────────┬────────┘
                                             │
                    ┌────────────────────────┼────────────────────────┐
                    │                        │                        │
              ┌─────▼─────┐            ┌─────▼─────┐            ┌─────▼─────┐
              │  Crypto   │            │Obfuscation│            │  Probing  │
              │  Engine   │            │  Engine   │            │ Resistance│
              └─────┬─────┘            └─────┬─────┘            └─────┬─────┘
                    │                        │                        │
                    └────────────────────────┼────────────────────────┘
                                             │
                                    ┌────────▼────────┐
                                    │   Transport     │
                                    │  Abstraction    │
                                    └────────┬────────┘
                                             │
              ┌──────────┬──────────┬────────┼────────┬──────────┬──────────┐
              │          │          │        │        │          │          │
           ┌──▼──┐   ┌───▼──┐   ┌───▼──┐  ┌──▼──┐  ┌──▼──┐   ┌───▼──┐   ┌───▼──┐
           │ UDP │   │ TCP  │   │  WS  │  │HTTP2│  │QUIC │   │ ICMP │   │ DNS  │
           └─────┘   └──────┘   └──────┘  └─────┘  └─────┘   └──────┘   └──────┘
```

### 3.2 Data Flow

```
Application Data
       │
       ▼
┌──────────────────┐
│ Chunk/Fragment   │  Split into chunks, compute Merkle hashes
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Stream Framing   │  Add stream ID, sequence numbers, offsets
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Compression      │  Optional LZ4 compression (if beneficial)
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ AEAD Encryption  │  XChaCha20-Poly1305 with ratcheted keys
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Padding          │  Match target size distribution
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Outer Framing    │  Add CID, wire format encoding
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Timing Queue     │  Hold for timing distribution matching
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Transport Send   │  UDP/TCP/WS/etc.
└──────────────────┘
```

---

## 4. Wire Protocol Specification

### 4.1 Polymorphic Wire Format

v2 introduces **session-negotiated wire formats** to prevent static fingerprinting. The wire format is derived from the session secret during handshake.

#### 4.1.1 Wire Format Derivation

```
Wire Format Seed:
    format_seed = HKDF-Expand(session_secret, "wire-format-v2", 32)

Field Order Derivation:
    field_rng = ChaCha20Rng::from_seed(format_seed)
    field_order = shuffle([CID, PAYLOAD, TAG, DUMMY...], field_rng)
    field_sizes = derive_field_sizes(field_rng)
```

#### 4.1.2 Possible Wire Format Configurations

The protocol supports multiple wire format configurations, randomly selected per session:

**Configuration A (QUIC-like):**
```
┌────────────────────────────────────────────────────────────────┐
│ CID (8B) │ Encrypted Payload (var) │ Auth Tag (16B)           │
└────────────────────────────────────────────────────────────────┘
```

**Configuration B (TLS-like):**
```
┌────────────────────────────────────────────────────────────────┐
│ Type (1B) │ Version (2B) │ Length (2B) │ Encrypted (var)      │
└────────────────────────────────────────────────────────────────┘
```

**Configuration C (Custom with dummy fields):**
```
┌────────────────────────────────────────────────────────────────┐
│ Dummy (3B) │ CID (6B) │ Dummy (2B) │ Payload (var) │ Tag (16B)│
└────────────────────────────────────────────────────────────────┘
```

#### 4.1.3 Wire Format Specification Structure

```rust
/// Wire format specification - negotiated per session
pub struct WireFormatSpec {
    /// Protocol version
    pub version: u8,
    
    /// Field definitions in wire order
    pub fields: Vec<WireField>,
    
    /// Total fixed overhead (sum of fixed field sizes)
    pub fixed_overhead: usize,
    
    /// Minimum packet size
    pub min_packet_size: usize,
    
    /// Maximum packet size
    pub max_packet_size: usize,
}

/// Individual field in wire format
pub enum WireField {
    /// Connection identifier
    ConnectionId {
        offset: usize,
        length: usize,  // 4-8 bytes
    },
    
    /// Authentication tag
    AuthTag {
        offset: usize,
        length: usize,  // 16 bytes standard
    },
    
    /// Encrypted payload (variable length)
    Payload {
        offset: usize,
        length_field: Option<LengthField>,
    },
    
    /// Dummy bytes (random, ignored)
    Dummy {
        offset: usize,
        length: usize,
    },
    
    /// Length indicator
    Length {
        offset: usize,
        length: usize,  // 1-4 bytes
        endian: Endianness,
        includes_self: bool,
    },
    
    /// Protocol version/type indicator
    Version {
        offset: usize,
        value: Vec<u8>,  // Fixed value for mimicry
    },
}
```

### 4.2 Inner Frame Format

After decryption, the inner frame has a consistent structure regardless of wire format:

```
Inner Frame (Post-Decryption):
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤
│                        Nonce (64 bits)                        │
│                                                               │
├───────────────────────────────────────────────────────────────┤
│  Frame Type │    Flags    │         Stream ID                 │
│   (8 bits)  │   (8 bits)  │         (16 bits)                 │
├───────────────────────────────────────────────────────────────┤
│                    Sequence Number (32 bits)                  │
├───────────────────────────────────────────────────────────────┤
│                    Payload Offset (64 bits)                   │
│                                                               │
├───────────────────────────────────────────────────────────────┤
│       Payload Length      │    Extension Count │ Extensions  │
│        (16 bits)          │      (8 bits)      │  (variable) │
├───────────────────────────────────────────────────────────────┤
│                    Payload Data (variable)                    │
│                           ...                                 │
├───────────────────────────────────────────────────────────────┤
│                    Padding (random, variable)                 │
└───────────────────────────────────────────────────────────────┘

Fixed Header: 24 bytes (without extensions)
```

### 4.3 Frame Types

| Value | Type | Description | v2 New |
|-------|------|-------------|--------|
| `0x00` | RESERVED | Invalid | |
| `0x01` | DATA | File/stream data | |
| `0x02` | ACK | Selective acknowledgment | |
| `0x03` | CONTROL | Stream management | |
| `0x04` | REKEY | Forward secrecy ratchet | |
| `0x05` | PING | Keepalive/RTT | |
| `0x06` | PONG | Ping response | |
| `0x07` | CLOSE | Session termination | |
| `0x08` | PAD | Cover traffic | |
| `0x09` | STREAM_OPEN | New stream | |
| `0x0A` | STREAM_CLOSE | Stream termination | |
| `0x0B` | STREAM_RESET | Abort stream | |
| `0x0C` | WINDOW_UPDATE | Flow control | |
| `0x0D` | GOAWAY | Graceful shutdown | |
| `0x0E` | PATH_CHALLENGE | Migration | |
| `0x0F` | PATH_RESPONSE | Migration ack | |
| `0x10` | GROUP_JOIN | Join group session | ✓ |
| `0x11` | GROUP_LEAVE | Leave group | ✓ |
| `0x12` | GROUP_REKEY | Group key update | ✓ |
| `0x13` | QOS_UPDATE | QoS parameters | ✓ |
| `0x14` | FEC_REPAIR | Forward error correction | ✓ |
| `0x15` | PRIORITY | Stream priority update | ✓ |
| `0x16` | DATAGRAM | Unreliable datagram | ✓ |
| `0x17` | TIMESTAMP | High-precision timing | ✓ |
| `0x18-0x1F` | RESERVED | Future use | |
| `0x20-0x3F` | EXTENSION | Application-defined | |

### 4.4 Frame Flags

```
Flags Byte (v2 Extended):
 7   6   5   4   3   2   1   0
├───┼───┼───┼───┼───┼───┼───┼───┤
│ECN│RTX│EXT│CMP│PRI│ACK│FIN│SYN│
└───┴───┴───┴───┴───┴───┴───┴───┘

Bit 0 (SYN): Stream synchronization
Bit 1 (FIN): Final frame in stream
Bit 2 (ACK): Acknowledgment data present
Bit 3 (PRI): Priority/expedited
Bit 4 (CMP): Compressed payload (LZ4)
Bit 5 (EXT): Extensions present
Bit 6 (RTX): Retransmission
Bit 7 (ECN): ECN feedback present
```

---

## 5. Cryptographic Protocol

### 5.1 Algorithm Suite

#### 5.1.1 Primary Suite (Default)

| Function | Algorithm | Security | Notes |
|----------|-----------|----------|-------|
| Classical KEM | X25519 | 128-bit | Standard ECDH |
| Post-Quantum KEM | ML-KEM-768 | 128-bit PQ | NIST FIPS 203 |
| AEAD | XChaCha20-Poly1305 | 256-bit | Extended nonce |
| Hash | BLAKE3 | 256-bit | Keyed/unkeyed modes |
| KDF | HKDF-BLAKE3 | 256-bit | Extract-then-expand |
| Signatures | Ed25519 | 128-bit | Identity only |
| Noise Hash | BLAKE2s | 128-bit | Noise framework requirement |

#### 5.1.2 Alternative Suites

**Suite B (Hardware Acceleration):**
| Function | Algorithm | Notes |
|----------|-----------|-------|
| Classical KEM | X25519 | |
| AEAD | AES-256-GCM | AES-NI required |
| Hash | SHA-256 | SHA-NI acceleration |

**Suite C (Maximum Security):**
| Function | Algorithm | Notes |
|----------|-----------|-------|
| Classical KEM | X448 | 224-bit security |
| Post-Quantum KEM | ML-KEM-1024 | 192-bit PQ security |
| AEAD | XChaCha20-Poly1305 | |
| Hash | BLAKE3 | |

### 5.2 Hybrid Key Exchange

v2 implements hybrid key exchange combining classical ECDH with post-quantum KEM:

```
Hybrid Key Exchange:
┌────────────────────────────────────────────────────────────────────────┐
│                                                                        │
│  Classical Component (X25519):                                        │
│    initiator_eph_sk, initiator_eph_pk = X25519_KeyGen()              │
│    classical_ss = X25519_DH(initiator_eph_sk, responder_eph_pk)      │
│                                                                        │
│  Post-Quantum Component (ML-KEM-768):                                 │
│    pq_ct, pq_ss = ML_KEM_Encaps(responder_pq_pk)                     │
│                                                                        │
│  Combined Secret:                                                      │
│    combined_ss = BLAKE3(                                              │
│        domain = "wraith-hybrid-kem-v2",                               │
│        input = classical_ss || pq_ss                                  │
│    )                                                                   │
│                                                                        │
│  Security: Secure if EITHER X25519 OR ML-KEM is secure               │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### 5.3 Noise_XX Handshake (Extended)

```
Extended Noise_XX with Hybrid KEM:

    Initiator (I)                           Responder (R)
    ─────────────────────────────────────────────────────────────
    s, e, pq_eph                            s, e, pq_eph
    ─────────────────────────────────────────────────────────────
    
    Phase 1: Initiator Hello (with probing resistance)
    ────────────────────────────────────────────────────────────────────
    → proof, e, [pq_pk]                    [~96 bytes + PQ overhead]
    
      proof = HMAC-BLAKE3(responder_pk, timestamp || random)
      e = Elligator2(ephemeral_public_key)
      pq_pk = ML-KEM-768 encapsulation key (optional, based on suite)
    
    Phase 2: Responder Response
    ────────────────────────────────────────────────────────────────────
    ← e, ee, [pq_ct], s, es               [~128 bytes + PQ overhead]
    
      e = Elligator2(responder_ephemeral)
      ee = DH(ie, re)
      pq_ct = ML-KEM encapsulation (if PQ enabled)
      s = Encrypted responder static key
      es = DH(ie, rs)
    
    Phase 3: Initiator Auth
    ────────────────────────────────────────────────────────────────────
    → s, se, [extensions]                  [~80 bytes]
    
      s = Encrypted initiator static key
      se = DH(is, re)
      extensions = Negotiated parameters (wire format, features, etc.)
    
    ═══════════════════════════════════════════════════════════════════
    Session Established: Symmetric keys derived from all DH outputs
```

### 5.4 Key Derivation

```
Session Key Derivation:

Input Keying Material (IKM):
┌────────────────────────────────────────────────────────────────────────┐
│  DH(ie, re)     [32 bytes]  - Ephemeral-ephemeral                     │
│  DH(ie, rs)     [32 bytes]  - Initiator ephemeral, Responder static   │
│  DH(is, re)     [32 bytes]  - Initiator static, Responder ephemeral   │
│  DH(is, rs)     [32 bytes]  - Static-static                           │
│  PQ_SS          [32 bytes]  - Post-quantum shared secret (if enabled) │
├────────────────────────────────────────────────────────────────────────┤
│  Total: 128-160 bytes                                                  │
└────────────────────────────────────────────────────────────────────────┘

Derivation:
    PRK = HKDF-Extract(
        salt = "wraith-v2-" || protocol_version,
        IKM = concatenated DH outputs
    )
    
    // Directional keys (different for each direction)
    initiator_send_key = HKDF-Expand(PRK, "i2r-data" || session_id, 32)
    responder_send_key = HKDF-Expand(PRK, "r2i-data" || session_id, 32)
    
    // Nonce salts
    initiator_nonce_salt = HKDF-Expand(PRK, "i2r-nonce", 4)
    responder_nonce_salt = HKDF-Expand(PRK, "r2i-nonce", 4)
    
    // Session identifiers
    session_id = HKDF-Expand(PRK, "session-id", 16)
    connection_id = HKDF-Expand(PRK, "connection-id", 8)
    
    // Wire format seed
    wire_format_seed = HKDF-Expand(PRK, "wire-format", 32)
    
    // Padding seed
    padding_seed = HKDF-Expand(PRK, "padding", 32)
```

### 5.5 Forward Secrecy Ratcheting

#### 5.5.1 Symmetric Ratchet (Per-Packet)

```rust
/// Per-packet key derivation using symmetric ratchet
pub struct SymmetricRatchet {
    chain_key: Zeroizing<[u8; 32]>,
    counter: u64,
}

impl SymmetricRatchet {
    /// Derive next message key and advance chain
    pub fn next(&mut self) -> MessageKey {
        // Derive message key
        let message_key = blake3::keyed_hash(
            &self.chain_key,
            &[0x02, /* counter bytes */]
        );
        
        // Advance chain key
        self.chain_key = Zeroizing::new(
            blake3::keyed_hash(&self.chain_key, &[0x01]).into()
        );
        
        self.counter += 1;
        
        MessageKey(message_key.into())
    }
}
```

#### 5.5.2 DH Ratchet (Periodic)

```
DH Ratchet Triggers:
├── Time-based: Every 120 seconds (default)
├── Volume-based: Every 1,000,000 packets
└── Event-based: On explicit REKEY request

REKEY Frame:
┌────────────────────────────────────────────────────────────────────────┐
│  New Ephemeral Public Key [Elligator2] (32 bytes)                     │
│  New PQ Encapsulation Key (optional, 1184 bytes for ML-KEM-768)       │
│  Ratchet Sequence Number (4 bytes)                                     │
│  Auth Tag (16 bytes)                                                   │
└────────────────────────────────────────────────────────────────────────┘

Ratchet Process:
    1. Generate new ephemeral keypair (X25519 + optional ML-KEM)
    2. Compute new DH shared secret
    3. Derive new chain key: chain_key' = HKDF(chain_key || new_dh, "ratchet")
    4. Zeroize old ephemeral private key immediately
    5. Continue with new keys
```

### 5.6 Cryptographic Implementation Requirements

#### 5.6.1 Constant-Time Operations

All cryptographic operations MUST be constant-time:

```rust
/// Constant-time byte comparison
#[inline(never)]
pub fn ct_eq(a: &[u8], b: &[u8]) -> subtle::Choice {
    use subtle::ConstantTimeEq;
    a.ct_eq(b)
}

/// Constant-time conditional select
pub fn ct_select<T: subtle::ConditionallySelectable>(
    condition: subtle::Choice,
    if_true: &T,
    if_false: &T,
) -> T {
    T::conditional_select(if_false, if_true, condition)
}
```

#### 5.6.2 Secure Memory Handling

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Sensitive key material with automatic zeroization
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretKey {
    bytes: [u8; 32],
}

/// Memory-locked buffer for highly sensitive data
pub struct LockedBuffer {
    ptr: *mut u8,
    len: usize,
}

impl LockedBuffer {
    pub fn new(size: usize) -> Result<Self, Error> {
        // Allocate
        let layout = Layout::from_size_align(size, 4096)?;
        let ptr = unsafe { alloc(layout) };
        
        // Lock in physical memory (prevent swap)
        #[cfg(unix)]
        unsafe {
            if libc::mlock(ptr as *const _, size) != 0 {
                return Err(Error::MemoryLock);
            }
        }
        
        // Guard pages (optional additional protection)
        #[cfg(unix)]
        unsafe {
            libc::madvise(ptr as *mut _, size, libc::MADV_DONTDUMP);
        }
        
        Ok(Self { ptr, len: size })
    }
}

impl Drop for LockedBuffer {
    fn drop(&mut self) {
        // Zeroize
        unsafe {
            std::ptr::write_bytes(self.ptr, 0, self.len);
        }
        std::sync::atomic::fence(Ordering::SeqCst);
        
        // Unlock
        #[cfg(unix)]
        unsafe {
            libc::munlock(self.ptr as *const _, self.len);
        }
        
        // Deallocate
        unsafe {
            dealloc(self.ptr, Layout::from_size_align_unchecked(self.len, 4096));
        }
    }
}
```

---

## 6. Traffic Obfuscation System

### 6.1 Design Goals

The obfuscation system aims to make WRAITH traffic computationally indistinguishable from:
1. Uniform random noise
2. Legitimate HTTPS/TLS traffic
3. Other specified protocols (WebSocket, HTTP/2, etc.)

### 6.2 Elligator2 Key Encoding

All public keys transmitted during handshake are Elligator2-encoded:

```rust
/// Generate Elligator2-encodable keypair
pub fn generate_elligator_keypair() -> (SecretKey, Representative) {
    loop {
        // Generate random scalar
        let secret = SecretKey::random(&mut OsRng);
        let public = PublicKey::from(&secret);
        
        // Attempt Elligator2 inverse mapping
        // ~50% of points are encodable
        if let Some(mut repr) = elligator2::encode(&public) {
            // Randomize high bit (not used by decoding)
            if OsRng.next_u32() & 1 == 1 {
                repr[31] |= 0x80;
            }
            return (secret, repr);
        }
    }
}

/// Decode representative back to public key
pub fn decode_representative(repr: &Representative) -> PublicKey {
    let mut clean = *repr;
    clean[31] &= 0x7F;  // Clear high bit
    elligator2::decode(&clean)
}
```

### 6.3 Continuous Padding Distribution

v2 replaces fixed padding classes with continuous distributions:

```rust
/// Padding distribution configuration
pub enum PaddingDistribution {
    /// Uniform distribution (maximum entropy)
    Uniform {
        min_size: u16,
        max_size: u16,
    },
    
    /// Match empirical HTTPS traffic distribution
    HttpsEmpirical,
    
    /// Gaussian distribution around target size
    Gaussian {
        mean: u16,
        std_dev: u16,
    },
    
    /// Application-specific profile
    Profile(TrafficProfile),
    
    /// Adaptive: learn from observed network traffic
    Adaptive {
        learning_rate: f32,
        window_size: usize,
    },
}

/// Sample packet size from distribution
pub fn sample_packet_size(
    dist: &PaddingDistribution,
    min_required: usize,
    rng: &mut impl CryptoRng,
) -> usize {
    match dist {
        PaddingDistribution::Uniform { min_size, max_size } => {
            let min = (*min_size as usize).max(min_required);
            let max = *max_size as usize;
            if min >= max { min } else { rng.gen_range(min..=max) }
        }
        
        PaddingDistribution::HttpsEmpirical => {
            // Empirical HTTPS packet size CDF (from traffic studies)
            // ~40% small (64-256), ~35% medium (256-800), ~25% large (800-1460)
            let r: f64 = rng.gen();
            let size = if r < 0.40 {
                rng.gen_range(64..256)
            } else if r < 0.75 {
                rng.gen_range(256..800)
            } else {
                rng.gen_range(800..1460)
            };
            size.max(min_required)
        }
        
        PaddingDistribution::Gaussian { mean, std_dev } => {
            use rand_distr::{Distribution, Normal};
            let normal = Normal::new(*mean as f64, *std_dev as f64).unwrap();
            let size = normal.sample(rng).round().max(min_required as f64) as usize;
            size.min(MAX_PACKET_SIZE)
        }
        
        // ... other distributions
    }
}
```

### 6.4 Timing Obfuscation

#### 6.4.1 Timing Distribution Matching

```rust
/// Timing obfuscation using learned patterns
pub struct TimingObfuscator {
    /// Current timing mode
    mode: TimingMode,
    
    /// Timing state machine
    state: TimingState,
    
    /// Last packet send time
    last_send: Instant,
    
    /// Accumulated timing credits
    timing_credits: Duration,
}

pub enum TimingMode {
    /// No timing obfuscation (minimum latency)
    Disabled,
    
    /// Match HTTPS browsing patterns
    HttpsBrowsing {
        /// Mean inter-request time
        mean_request_interval: Duration,
        /// Request burst size distribution
        burst_size: (usize, usize),
    },
    
    /// Match video streaming patterns
    VideoStreaming {
        /// Segment duration
        segment_interval: Duration,
        /// Bitrate (affects packet rate)
        bitrate_bps: u64,
    },
    
    /// Constant-rate transmission
    ConstantRate {
        /// Target packets per second
        packets_per_second: f64,
    },
    
    /// Custom Hidden Markov Model
    CustomHmm {
        model: TimingHmm,
    },
}

impl TimingObfuscator {
    /// Calculate next packet send time
    pub fn next_send_time(&mut self, has_data: bool) -> Instant {
        let base_delay = match &self.mode {
            TimingMode::Disabled => Duration::ZERO,
            
            TimingMode::ConstantRate { packets_per_second } => {
                Duration::from_secs_f64(1.0 / packets_per_second)
            }
            
            TimingMode::HttpsBrowsing { mean_request_interval, .. } => {
                // Exponential distribution
                let lambda = 1.0 / mean_request_interval.as_secs_f64();
                let delay = -lambda.recip() * self.rng.gen::<f64>().ln();
                Duration::from_secs_f64(delay)
            }
            
            // ... other modes
        };
        
        // Add jitter
        let jitter = self.sample_jitter();
        
        self.last_send + base_delay + jitter
    }
}
```

### 6.5 Active Probing Resistance

#### 6.5.1 Proof-of-Knowledge Requirement

```rust
/// Probing resistance configuration
pub struct ProbingResistance {
    /// Require proof of server knowledge in first packet
    pub require_proof: bool,
    
    /// How to respond to invalid probes
    pub probe_response: ProbeResponse,
    
    /// Optional service fronting
    pub fronting: Option<FrontingConfig>,
}

pub enum ProbeResponse {
    /// Silent drop (simplest, but fingerprintable by timeout)
    SilentDrop,
    
    /// Mimic TLS server
    MimicTls {
        /// Certificate to present
        certificate: Certificate,
        /// Supported cipher suites to advertise
        cipher_suites: Vec<CipherSuite>,
    },
    
    /// Mimic HTTP server
    MimicHttp {
        /// Server header
        server_header: String,
        /// Response for /
        index_page: Vec<u8>,
    },
    
    /// Proxy to real service
    ProxyToBackend {
        backend: SocketAddr,
        protocol_marker: ProtocolMarker,
    },
}

/// Proof of server knowledge in initial packet
pub fn compute_client_proof(
    server_public_key: &PublicKey,
    timestamp: u64,
    random: &[u8; 16],
) -> [u8; 32] {
    blake3::keyed_hash(
        server_public_key.as_bytes(),
        &[&timestamp.to_be_bytes()[..], random].concat(),
    ).into()
}

/// Verify client proof
pub fn verify_client_proof(
    server_secret_key: &SecretKey,
    proof: &[u8; 32],
    timestamp: u64,
    random: &[u8; 16],
    max_clock_skew: Duration,
) -> bool {
    // Check timestamp freshness
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    if (now as i64 - timestamp as i64).abs() > max_clock_skew.as_secs() as i64 {
        return false;
    }
    
    // Verify proof
    let server_public_key = PublicKey::from(server_secret_key);
    let expected = compute_client_proof(&server_public_key, timestamp, random);
    
    constant_time::ct_eq(proof, &expected).into()
}
```

#### 6.5.2 Service Fronting

```rust
/// Service fronting configuration
pub struct FrontingConfig {
    /// Real backend service
    pub backend: SocketAddr,
    
    /// How to identify protocol traffic vs. legitimate traffic
    pub marker: ProtocolMarker,
    
    /// Content to serve for non-protocol requests
    pub cover_content: CoverContent,
}

pub enum ProtocolMarker {
    /// Secret path: /api/v2/sync/{base64(HMAC(secret, session_random))}
    HttpPath {
        path_prefix: String,
        hmac_key: [u8; 32],
    },
    
    /// Secret header value
    HttpHeader {
        header_name: String,
        hmac_key: [u8; 32],
    },
    
    /// TLS SNI pattern
    TlsSni {
        domain_pattern: String,  // e.g., "*.cdn.example.com"
    },
    
    /// First N bytes of connection
    InitialBytes {
        pattern: Vec<u8>,
        offset: usize,
    },
}

/// Handle incoming connection with fronting
pub async fn handle_fronted_connection(
    conn: TcpStream,
    config: &FrontingConfig,
) -> Result<(), Error> {
    // Peek at initial bytes
    let mut peek_buf = [0u8; 1024];
    let n = conn.peek(&mut peek_buf).await?;
    
    // Check for protocol marker
    if is_protocol_traffic(&peek_buf[..n], &config.marker) {
        // Handle as WRAITH protocol
        handle_protocol_connection(conn).await
    } else {
        // Proxy to real backend
        proxy_to_backend(conn, config.backend).await
    }
}
```

### 6.6 Entropy Normalization

```rust
/// Normalize ciphertext entropy to match target protocol
pub enum EntropyNormalization {
    /// No normalization (raw ciphertext)
    None,
    
    /// Insert predictable bytes at derived positions
    PredictableInsertion {
        /// Insertion ratio (e.g., 0.05 = 5% overhead)
        ratio: f32,
    },
    
    /// Encode as Base64 (reduces entropy to ~6 bits/byte)
    Base64,
    
    /// Encode as printable ASCII
    PrintableAscii,
    
    /// Wrap in JSON structure
    JsonWrapper {
        template: JsonTemplate,
    },
    
    /// Wrap in HTTP chunked encoding
    HttpChunked,
}

impl EntropyNormalization {
    /// Encode ciphertext
    pub fn encode(&self, ciphertext: &[u8], key: &[u8; 32]) -> Vec<u8> {
        match self {
            Self::None => ciphertext.to_vec(),
            
            Self::PredictableInsertion { ratio } => {
                // Derive insertion positions from key
                let mut rng = ChaCha20Rng::from_seed(*key);
                let insert_count = (ciphertext.len() as f32 * ratio) as usize;
                let mut positions: Vec<usize> = (0..ciphertext.len())
                    .collect::<Vec<_>>()
                    .choose_multiple(&mut rng, insert_count)
                    .cloned()
                    .collect();
                positions.sort();
                
                // Insert predictable bytes
                let mut output = Vec::with_capacity(ciphertext.len() + insert_count);
                let mut ct_idx = 0;
                let mut pos_idx = 0;
                let mut output_idx = 0;
                
                while ct_idx < ciphertext.len() {
                    if pos_idx < positions.len() && output_idx == positions[pos_idx] {
                        // Insert predictable byte
                        output.push(rng.gen());
                        pos_idx += 1;
                    } else {
                        output.push(ciphertext[ct_idx]);
                        ct_idx += 1;
                    }
                    output_idx += 1;
                }
                
                output
            }
            
            Self::JsonWrapper { template } => {
                // Wrap ciphertext in innocent-looking JSON
                let b64 = base64::encode(ciphertext);
                template.render(&b64)
            }
            
            // ... other encodings
        }
    }
}
```

### 6.7 Decoy Traffic

```rust
/// Decoy traffic configuration
pub struct DecoyTrafficConfig {
    /// Enable decoy streams
    pub enabled: bool,
    
    /// Number of decoy streams to maintain
    pub stream_count: usize,
    
    /// Bandwidth allocated to decoys
    pub bandwidth: Bandwidth,
    
    /// How decoy data is generated
    pub content_generator: DecoyContentGenerator,
    
    /// Mixing strategy with real data
    pub mixing_strategy: MixingStrategy,
}

pub enum DecoyContentGenerator {
    /// Pure random bytes
    Random,
    
    /// Compressible random (mimics real file data)
    CompressibleRandom {
        target_ratio: f32,
    },
    
    /// Pattern from captured traffic
    ReplayPattern {
        patterns: Vec<TrafficPattern>,
    },
}

pub enum MixingStrategy {
    /// Decoys replaced by real data (constant total bandwidth)
    Replace,
    
    /// Real data added on top of decoys (variable bandwidth)
    Additive,
    
    /// Real and decoy interleaved (uniform appearance)
    Interleave,
}

/// Decoy traffic generator
pub struct DecoyGenerator {
    config: DecoyTrafficConfig,
    streams: Vec<DecoyStream>,
    rng: ChaCha20Rng,
}

impl DecoyGenerator {
    /// Generate next decoy packet
    pub fn next_packet(&mut self) -> Option<Packet> {
        if !self.config.enabled {
            return None;
        }
        
        // Select random stream
        let stream = self.streams.choose_mut(&mut self.rng)?;
        
        // Generate content based on strategy
        let content = match &self.config.content_generator {
            DecoyContentGenerator::Random => {
                let size = self.rng.gen_range(64..1400);
                let mut buf = vec![0u8; size];
                self.rng.fill_bytes(&mut buf);
                buf
            }
            // ... other generators
        };
        
        Some(Packet {
            stream_id: stream.id,
            payload: content,
            is_decoy: true,
        })
    }
}
```

---

## 7. Transport Abstraction Layer

### 7.1 Transport Trait

```rust
/// Abstract transport interface
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Send a packet
    async fn send(&self, packet: &[u8]) -> Result<(), TransportError>;
    
    /// Receive a packet
    async fn recv(&self, buf: &mut [u8]) -> Result<usize, TransportError>;
    
    /// Get transport characteristics
    fn characteristics(&self) -> TransportCharacteristics;
    
    /// Get effective MTU
    fn mtu(&self) -> usize;
    
    /// Local address
    fn local_addr(&self) -> Result<SocketAddr, TransportError>;
    
    /// Remote address (if connected)
    fn peer_addr(&self) -> Option<SocketAddr>;
    
    /// Close transport
    async fn close(&self) -> Result<(), TransportError>;
}

/// Transport characteristics
#[derive(Clone, Debug)]
pub struct TransportCharacteristics {
    /// Is transport reliable (handles retransmission)?
    pub reliable: bool,
    
    /// Is transport ordered?
    pub ordered: bool,
    
    /// Underlying protocol
    pub protocol: TransportProtocol,
    
    /// Added latency
    pub base_latency: Duration,
    
    /// Per-packet overhead
    pub overhead: usize,
    
    /// Supports datagrams?
    pub datagram_capable: bool,
    
    /// Maximum bandwidth (if known/limited)
    pub max_bandwidth: Option<Bandwidth>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportProtocol {
    Udp,
    Tcp,
    WebSocket,
    Http2,
    Http3,
    Quic,
    Icmp,
    Dns,
    RawEthernet,
    Custom,
}
```

### 7.2 Transport Implementations

#### 7.2.1 UDP Transport

```rust
pub struct UdpTransport {
    socket: UdpSocket,
    peer_addr: SocketAddr,
}

#[async_trait]
impl Transport for UdpTransport {
    async fn send(&self, packet: &[u8]) -> Result<(), TransportError> {
        self.socket.send_to(packet, self.peer_addr).await?;
        Ok(())
    }
    
    async fn recv(&self, buf: &mut [u8]) -> Result<usize, TransportError> {
        let (n, _) = self.socket.recv_from(buf).await?;
        Ok(n)
    }
    
    fn characteristics(&self) -> TransportCharacteristics {
        TransportCharacteristics {
            reliable: false,
            ordered: false,
            protocol: TransportProtocol::Udp,
            base_latency: Duration::ZERO,
            overhead: 8,  // UDP header
            datagram_capable: true,
            max_bandwidth: None,
        }
    }
    
    fn mtu(&self) -> usize {
        1472  // 1500 - 20 (IP) - 8 (UDP)
    }
    
    // ... other methods
}
```

#### 7.2.2 WebSocket Transport

```rust
pub struct WebSocketTransport {
    ws: WebSocketStream<TcpStream>,
    read_buf: VecDeque<u8>,
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn send(&self, packet: &[u8]) -> Result<(), TransportError> {
        // Send as binary WebSocket message
        self.ws.send(Message::Binary(packet.to_vec())).await?;
        Ok(())
    }
    
    async fn recv(&self, buf: &mut [u8]) -> Result<usize, TransportError> {
        loop {
            if let Some(msg) = self.ws.next().await {
                match msg? {
                    Message::Binary(data) => {
                        let len = data.len().min(buf.len());
                        buf[..len].copy_from_slice(&data[..len]);
                        return Ok(len);
                    }
                    Message::Ping(data) => {
                        self.ws.send(Message::Pong(data)).await?;
                    }
                    Message::Close(_) => {
                        return Err(TransportError::Closed);
                    }
                    _ => continue,
                }
            } else {
                return Err(TransportError::Closed);
            }
        }
    }
    
    fn characteristics(&self) -> TransportCharacteristics {
        TransportCharacteristics {
            reliable: true,   // TCP-based
            ordered: true,    // TCP-based
            protocol: TransportProtocol::WebSocket,
            base_latency: Duration::from_millis(1),
            overhead: 2 + 4,  // WS header + mask
            datagram_capable: true,  // Binary messages
            max_bandwidth: None,
        }
    }
    
    fn mtu(&self) -> usize {
        65535 - 14  // WebSocket frame max - header
    }
}
```

#### 7.2.3 Kernel Bypass Transport (AF_XDP)

```rust
#[cfg(target_os = "linux")]
pub struct AfXdpTransport {
    socket: XskSocket,
    umem: Umem,
    fill_queue: FillQueue,
    comp_queue: CompQueue,
    tx_queue: TxQueue,
    rx_queue: RxQueue,
}

#[cfg(target_os = "linux")]
#[async_trait]
impl Transport for AfXdpTransport {
    async fn send(&self, packet: &[u8]) -> Result<(), TransportError> {
        // Zero-copy send using UMEM
        let frame = self.umem.alloc_frame()?;
        frame.copy_from_slice(packet);
        
        // Submit to TX queue
        self.tx_queue.submit(&[frame.addr()])?;
        
        // Kick the kernel
        self.socket.sendto()?;
        
        Ok(())
    }
    
    async fn recv(&self, buf: &mut [u8]) -> Result<usize, TransportError> {
        // Wait for packet
        self.socket.poll(PollFlags::POLLIN)?;
        
        // Receive from RX queue
        let descs = self.rx_queue.receive(1)?;
        if descs.is_empty() {
            return Err(TransportError::WouldBlock);
        }
        
        // Zero-copy read
        let frame = self.umem.frame(descs[0].addr);
        let len = descs[0].len.min(buf.len());
        buf[..len].copy_from_slice(&frame[..len]);
        
        // Return frame to fill queue
        self.fill_queue.submit(&[descs[0].addr])?;
        
        Ok(len)
    }
    
    fn characteristics(&self) -> TransportCharacteristics {
        TransportCharacteristics {
            reliable: false,
            ordered: false,
            protocol: TransportProtocol::RawEthernet,
            base_latency: Duration::from_nanos(100),  // Extremely low
            overhead: 14,  // Ethernet header
            datagram_capable: true,
            max_bandwidth: Some(Bandwidth::from_gbps(100)),
        }
    }
    
    fn mtu(&self) -> usize {
        9000 - 14  // Jumbo frame - Ethernet header
    }
}
```

### 7.3 Transport Selection

```rust
/// Select appropriate transport based on requirements
pub async fn select_transport(
    config: &TransportConfig,
    requirements: &TransportRequirements,
) -> Result<Box<dyn Transport>, Error> {
    // Priority order based on requirements
    let candidates: Vec<TransportProtocol> = if requirements.stealth_priority {
        // Prioritize common protocols that bypass firewalls
        vec![
            TransportProtocol::WebSocket,
            TransportProtocol::Http2,
            TransportProtocol::Tcp,
        ]
    } else if requirements.performance_priority {
        // Prioritize low-latency/high-throughput
        vec![
            TransportProtocol::RawEthernet,
            TransportProtocol::Udp,
            TransportProtocol::Quic,
        ]
    } else {
        // Balanced
        vec![
            TransportProtocol::Quic,
            TransportProtocol::Udp,
            TransportProtocol::WebSocket,
            TransportProtocol::Tcp,
        ]
    };
    
    for proto in candidates {
        if let Ok(transport) = try_create_transport(proto, config).await {
            // Verify transport works
            if verify_transport(&transport, requirements).await {
                return Ok(transport);
            }
        }
    }
    
    Err(Error::NoSuitableTransport)
}
```

---

## 8. Session Management

### 8.1 Session State Machine

```
                              ┌─────────────────┐
                              │     CLOSED      │
                              └────────┬────────┘
                                       │ connect()/accept()
                                       ▼
                        ┌──────────────────────────────┐
                        │         CONNECTING           │
                        │  ┌────────────────────────┐  │
                        │  │ Transport negotiation  │  │
                        │  │ Proof verification     │  │
                        │  │ Noise handshake        │  │
                        │  └────────────────────────┘  │
                        └──────────────┬───────────────┘
                                       │ handshake complete
                                       ▼
                              ┌─────────────────┐
                    ┌─────────│   ESTABLISHED   │◄────────────────┐
                    │         └────────┬────────┘                 │
                    │                  │                          │
         ┌──────────┼──────────────────┼────────────────┬─────────┤
         │          │                  │                │         │
         ▼          ▼                  ▼                ▼         │
   ┌───────────┐ ┌────────┐    ┌─────────────┐   ┌──────────┐    │
   │  REKEYING │ │DRAINING│    │  MIGRATING  │   │ RESUMING │    │
   │           │ │        │    │             │   │          │    │
   └─────┬─────┘ └────┬───┘    └──────┬──────┘   └────┬─────┘    │
         │            │               │               │          │
         │            │               │               │          │
         └────────────┴───────────────┴───────────────┴──────────┘
                                       │
                                       │ close/timeout
                                       ▼
                              ┌─────────────────┐
                              │     CLOSED      │
                              └─────────────────┘
```

### 8.2 Session Structure

```rust
/// WRAITH session
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    
    /// Connection identifier (rotates)
    pub connection_id: ConnectionId,
    
    /// Current state
    pub state: SessionState,
    
    /// Cryptographic state
    pub crypto: CryptoState,
    
    /// Active streams
    pub streams: HashMap<StreamId, Stream>,
    
    /// Flow control state
    pub flow_control: FlowControl,
    
    /// Congestion control state
    pub congestion: CongestionController,
    
    /// Transport binding
    pub transport: Box<dyn Transport>,
    
    /// Obfuscation configuration
    pub obfuscation: ObfuscationConfig,
    
    /// Resource profile
    pub profile: ResourceProfile,
    
    /// Timing obfuscator
    pub timing: TimingObfuscator,
    
    /// Statistics
    pub stats: SessionStats,
}

/// Session cryptographic state
pub struct CryptoState {
    /// Our static keypair
    pub static_keypair: Keypair,
    
    /// Peer's static public key
    pub peer_static: PublicKey,
    
    /// Current send chain (symmetric ratchet)
    pub send_chain: SymmetricRatchet,
    
    /// Current receive chain
    pub recv_chain: SymmetricRatchet,
    
    /// Current ephemeral keypair (for DH ratchet)
    pub ephemeral: Keypair,
    
    /// Peer's current ephemeral
    pub peer_ephemeral: PublicKey,
    
    /// Post-quantum state (if enabled)
    pub pq_state: Option<PqState>,
    
    /// Ratchet generation
    pub ratchet_gen: u32,
    
    /// Wire format for this session
    pub wire_format: WireFormatSpec,
}
```

### 8.3 Session Resumption

```rust
/// Resumption ticket for session resumption
#[derive(Serialize, Deserialize)]
pub struct ResumptionTicket {
    /// Ticket identifier
    pub id: [u8; 16],
    
    /// Session identifier
    pub session_id: SessionId,
    
    /// Resumption secret
    pub resumption_secret: [u8; 32],
    
    /// Server static key fingerprint
    pub server_fingerprint: [u8; 32],
    
    /// Ticket expiration
    pub expires: SystemTime,
    
    /// Encrypted session parameters
    pub encrypted_params: Vec<u8>,
}

impl Session {
    /// Create resumption ticket
    pub fn create_resumption_ticket(&self) -> ResumptionTicket {
        let resumption_secret = self.crypto.derive_resumption_secret();
        
        // Encrypt session parameters
        let params = SessionParams {
            wire_format: self.crypto.wire_format.clone(),
            obfuscation: self.obfuscation.clone(),
            negotiated_extensions: self.extensions.clone(),
        };
        
        let encrypted_params = encrypt_ticket_params(
            &params,
            &resumption_secret,
        );
        
        ResumptionTicket {
            id: OsRng.gen(),
            session_id: self.id,
            resumption_secret,
            server_fingerprint: self.crypto.peer_static.fingerprint(),
            expires: SystemTime::now() + Duration::from_secs(86400),
            encrypted_params,
        }
    }
    
    /// Resume from ticket
    pub async fn resume(
        ticket: &ResumptionTicket,
        transport: Box<dyn Transport>,
    ) -> Result<Self, Error> {
        // Verify ticket not expired
        if SystemTime::now() > ticket.expires {
            return Err(Error::TicketExpired);
        }
        
        // Send resumption request
        let request = ResumptionRequest {
            ticket_id: ticket.id,
            client_random: OsRng.gen(),
        };
        
        // Abbreviated handshake...
        todo!()
    }
}
```

---

## 9. Stream Multiplexing

### 9.1 Stream Types

```rust
/// Stream identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StreamId(pub u16);

impl StreamId {
    /// Stream type from ID
    pub fn stream_type(&self) -> StreamType {
        match self.0 {
            0 => StreamType::Control,
            0x0001..=0x3FFF => StreamType::ClientInitiated,
            0x4000..=0x7FFF => StreamType::ServerInitiated,
            0x8000..=0xBFFF => StreamType::ClientExpedited,
            0xC000..=0xFFFF => StreamType::ServerExpedited,
        }
    }
    
    /// Is expedited (priority) stream?
    pub fn is_expedited(&self) -> bool {
        self.0 >= 0x8000
    }
}

#[derive(Clone, Copy, Debug)]
pub enum StreamType {
    /// Stream 0: Session control
    Control,
    /// Client-initiated normal priority
    ClientInitiated,
    /// Server-initiated normal priority
    ServerInitiated,
    /// Client-initiated expedited
    ClientExpedited,
    /// Server-initiated expedited
    ServerExpedited,
}
```

### 9.2 Stream State Machine

```
                         ┌────────────────┐
                         │      IDLE      │
                         └───────┬────────┘
                                 │ STREAM_OPEN
                                 ▼
                         ┌────────────────┐
                         │      OPEN      │
                         └───────┬────────┘
                                 │
          ┌──────────────────────┼──────────────────────┐
          │                      │                      │
          │ send FIN             │ recv FIN             │ send RESET
          ▼                      ▼                      ▼
┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
│   HALF_CLOSED    │   │   HALF_CLOSED    │   │      RESET       │
│     (local)      │   │     (remote)     │   │                  │
└────────┬─────────┘   └────────┬─────────┘   └────────┬─────────┘
         │ recv FIN             │ send FIN             │
         │                      │                      │
         └──────────────────────┼──────────────────────┘
                                │
                                ▼
                         ┌────────────────┐
                         │     CLOSED     │
                         └────────────────┘
```

### 9.3 Stream Structure

```rust
/// Stream state
pub struct Stream {
    /// Stream identifier
    pub id: StreamId,
    
    /// Current state
    pub state: StreamState,
    
    /// QoS mode for this stream
    pub qos: QosMode,
    
    /// Priority (0-255, higher = more important)
    pub priority: u8,
    
    /// Send buffer
    pub send_buffer: SendBuffer,
    
    /// Receive buffer
    pub recv_buffer: ReceiveBuffer,
    
    /// Flow control window (send side)
    pub send_window: u64,
    
    /// Flow control window (receive side)
    pub recv_window: u64,
    
    /// Sequence numbers
    pub send_seq: u32,
    pub recv_seq: u32,
    
    /// Unacknowledged packets
    pub unacked: BTreeMap<u32, SentPacket>,
    
    /// Metadata (e.g., file info)
    pub metadata: Option<StreamMetadata>,
}

/// QoS modes for different use cases
#[derive(Clone, Copy, Debug)]
pub enum QosMode {
    /// Reliable, ordered delivery (default)
    Reliable,
    
    /// Unreliable, ordered (drop old packets)
    UnreliableOrdered {
        max_latency: Duration,
    },
    
    /// Unreliable, unordered
    UnreliableUnordered,
    
    /// Partially reliable (limited retransmits)
    PartiallyReliable {
        max_retransmits: u8,
        max_age: Duration,
    },
}
```

---

## 10. Flow Control and Congestion

### 10.1 Flow Control

```rust
/// Flow control state
pub struct FlowControl {
    /// Connection-level send window
    pub connection_send_window: u64,
    
    /// Connection-level receive window
    pub connection_recv_window: u64,
    
    /// Initial stream window size
    pub initial_stream_window: u64,
    
    /// Maximum stream window size
    pub max_stream_window: u64,
    
    /// Blocked streams (waiting for window)
    pub blocked_streams: HashSet<StreamId>,
}

impl FlowControl {
    /// Consume window (when sending data)
    pub fn consume(&mut self, stream_id: StreamId, bytes: u64) -> bool {
        // Check connection-level window
        if bytes > self.connection_send_window {
            return false;
        }
        
        // Check stream-level window
        if let Some(stream) = self.streams.get_mut(&stream_id) {
            if bytes > stream.send_window {
                self.blocked_streams.insert(stream_id);
                return false;
            }
            stream.send_window -= bytes;
        }
        
        self.connection_send_window -= bytes;
        true
    }
    
    /// Release window (when receiving WINDOW_UPDATE)
    pub fn release(
        &mut self,
        stream_id: Option<StreamId>,
        increment: u64,
    ) {
        if let Some(id) = stream_id {
            // Stream-level update
            if let Some(stream) = self.streams.get_mut(&id) {
                stream.send_window = stream.send_window
                    .saturating_add(increment)
                    .min(self.max_stream_window);
                self.blocked_streams.remove(&id);
            }
        } else {
            // Connection-level update
            self.connection_send_window = self.connection_send_window
                .saturating_add(increment)
                .min(MAX_CONNECTION_WINDOW);
        }
    }
}
```

### 10.2 Congestion Control (BBRv2)

```rust
/// BBRv2-inspired congestion controller
pub struct CongestionController {
    /// Estimated bottleneck bandwidth
    pub btl_bw: Bandwidth,
    
    /// Minimum RTT
    pub min_rtt: Duration,
    
    /// Smoothed RTT
    pub srtt: Duration,
    
    /// RTT variance
    pub rtt_var: Duration,
    
    /// Current state
    pub state: BbrState,
    
    /// Pacing gain
    pub pacing_gain: f64,
    
    /// Congestion window gain
    pub cwnd_gain: f64,
    
    /// In-flight bytes
    pub in_flight: u64,
    
    /// Congestion window
    pub cwnd: u64,
    
    /// Pacing rate
    pub pacing_rate: Bandwidth,
    
    /// Round trip counter
    pub round_count: u64,
    
    /// Loss state
    pub loss: LossState,
}

#[derive(Clone, Copy, Debug)]
pub enum BbrState {
    /// Initial exponential growth
    Startup,
    
    /// Drain queue after startup
    Drain,
    
    /// Steady-state bandwidth probing (8 phases)
    ProbeBw {
        phase: u8,
        cycle_start: Instant,
    },
    
    /// Periodic RTT measurement
    ProbeRtt {
        start: Instant,
    },
}

impl CongestionController {
    /// Update on ACK received
    pub fn on_ack(&mut self, ack: &AckInfo) {
        // Update bandwidth estimate
        let delivery_rate = ack.bytes_delivered as f64 / ack.elapsed.as_secs_f64();
        self.update_bandwidth(Bandwidth::from_bps(delivery_rate as u64));
        
        // Update RTT estimate
        self.update_rtt(ack.rtt);
        
        // Advance round
        if ack.is_round_end {
            self.round_count += 1;
        }
        
        // State machine transitions
        match self.state {
            BbrState::Startup => {
                if !self.bandwidth_growing() {
                    self.state = BbrState::Drain;
                    self.pacing_gain = 0.75;  // Drain queue
                }
            }
            
            BbrState::Drain => {
                if self.in_flight <= self.bdp() {
                    self.enter_probe_bw();
                }
            }
            
            BbrState::ProbeBw { phase, cycle_start } => {
                // Cycle through 8 phases
                if cycle_start.elapsed() > self.min_rtt {
                    let next_phase = (phase + 1) % 8;
                    self.pacing_gain = PACING_GAINS[next_phase as usize];
                    self.state = BbrState::ProbeBw {
                        phase: next_phase,
                        cycle_start: Instant::now(),
                    };
                }
                
                // Check if we need to probe RTT
                if self.should_probe_rtt() {
                    self.state = BbrState::ProbeRtt {
                        start: Instant::now(),
                    };
                }
            }
            
            BbrState::ProbeRtt { start } => {
                if start.elapsed() > Duration::from_millis(200) {
                    self.enter_probe_bw();
                }
            }
        }
        
        // Update cwnd
        self.cwnd = (self.bdp() as f64 * self.cwnd_gain) as u64;
        self.cwnd = self.cwnd.max(MIN_CWND);
        
        // Update pacing rate
        self.pacing_rate = Bandwidth::from_bps(
            (self.btl_bw.as_bps() as f64 * self.pacing_gain) as u64
        );
    }
    
    /// Bandwidth-delay product
    pub fn bdp(&self) -> u64 {
        (self.btl_bw.as_bps() as f64 * self.min_rtt.as_secs_f64() / 8.0) as u64
    }
}
```

### 10.3 Pacer

```rust
/// Packet pacer for smooth transmission
pub struct Pacer {
    /// Current pacing rate
    rate: Bandwidth,
    
    /// Token bucket
    tokens: f64,
    
    /// Maximum burst
    max_burst: usize,
    
    /// Last update time
    last_update: Instant,
}

impl Pacer {
    /// Get next send time for a packet
    pub fn next_send_time(&mut self, packet_size: usize) -> Instant {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        
        // Accumulate tokens
        let new_tokens = self.rate.as_bps() as f64 / 8.0 * elapsed.as_secs_f64();
        self.tokens = (self.tokens + new_tokens).min(self.max_burst as f64);
        self.last_update = now;
        
        if self.tokens >= packet_size as f64 {
            // Can send immediately
            self.tokens -= packet_size as f64;
            now
        } else {
            // Calculate wait time
            let needed = packet_size as f64 - self.tokens;
            let wait_secs = needed / (self.rate.as_bps() as f64 / 8.0);
            now + Duration::from_secs_f64(wait_secs)
        }
    }
}
```

---

## 11. NAT Traversal

### 11.1 Endpoint Discovery

```rust
/// Endpoint discovery using STUN-like protocol
pub struct EndpointDiscovery {
    /// Known relay servers
    relays: Vec<RelayInfo>,
    
    /// Discovered local endpoints
    local_endpoints: Vec<Endpoint>,
    
    /// Discovered server-reflexive endpoints
    reflexive_endpoints: Vec<Endpoint>,
}

impl EndpointDiscovery {
    /// Discover all endpoints
    pub async fn discover(&mut self) -> Result<Vec<Endpoint>, Error> {
        let mut endpoints = Vec::new();
        
        // Local addresses
        for addr in get_local_addresses()? {
            endpoints.push(Endpoint {
                addr,
                endpoint_type: EndpointType::Local,
                priority: 100,
            });
        }
        
        // Server-reflexive (STUN-style)
        for relay in &self.relays {
            if let Ok(mapped) = self.stun_request(relay).await {
                endpoints.push(Endpoint {
                    addr: mapped,
                    endpoint_type: EndpointType::ServerReflexive,
                    priority: 50,
                });
            }
        }
        
        // Sort by priority
        endpoints.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Ok(endpoints)
    }
}
```

### 11.2 Hole Punching

```rust
/// NAT hole punching coordinator
pub struct HolePuncher {
    /// Our endpoints
    local_endpoints: Vec<Endpoint>,
    
    /// Peer's endpoints
    remote_endpoints: Vec<Endpoint>,
    
    /// Signaling channel (via relay)
    signaling: SignalingChannel,
    
    /// Probe state
    probes: HashMap<(Endpoint, Endpoint), ProbeState>,
}

impl HolePuncher {
    /// Attempt to establish direct connection
    pub async fn punch(&mut self) -> Result<DirectPath, Error> {
        // Exchange endpoint candidates via signaling
        self.signaling.send(EndpointCandidates {
            endpoints: self.local_endpoints.clone(),
        }).await?;
        
        let remote = self.signaling.recv::<EndpointCandidates>().await?;
        self.remote_endpoints = remote.endpoints;
        
        // Try all endpoint pairs
        let pairs = self.generate_pairs();
        
        // Send probes to all pairs simultaneously
        for (local, remote) in &pairs {
            self.send_probe(local, remote).await?;
        }
        
        // Wait for first successful path
        let timeout = Duration::from_secs(5);
        let start = Instant::now();
        
        while start.elapsed() < timeout {
            if let Some(path) = self.check_for_path().await? {
                return Ok(path);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        
        Err(Error::HolePunchFailed)
    }
    
    /// Birthday attack for symmetric NAT
    pub async fn birthday_punch(&mut self) -> Result<DirectPath, Error> {
        // Open many local ports
        let local_ports: Vec<UdpSocket> = (0..256)
            .filter_map(|_| UdpSocket::bind("0.0.0.0:0").ok())
            .collect();
        
        // Predict remote port range based on observed mapping
        let predicted_range = self.predict_port_range()?;
        
        // Send probes from all local ports to predicted range
        for socket in &local_ports {
            for port in predicted_range.clone() {
                let addr = SocketAddr::new(
                    self.remote_endpoints[0].addr.ip(),
                    port,
                );
                socket.send_to(&self.create_probe(), addr).await?;
            }
        }
        
        // Wait for response
        self.wait_for_path(Duration::from_secs(10)).await
    }
}
```

### 11.3 Relay Protocol

```rust
/// DERP-style relay protocol
pub struct RelayProtocol {
    /// Relay connection
    conn: TcpStream,
    
    /// Our public key
    public_key: PublicKey,
    
    /// Encryption state
    encryption: Option<RelayEncryption>,
}

/// Relay frame types
#[repr(u8)]
pub enum RelayFrameType {
    /// Forward packet to peer
    SendPacket = 0x01,
    
    /// Receive packet from peer
    RecvPacket = 0x02,
    
    /// Register public key
    Subscribe = 0x03,
    
    /// Keepalive
    KeepAlive = 0x04,
    
    /// Peer notification
    PeerPresent = 0x05,
    PeerGone = 0x06,
}

impl RelayProtocol {
    /// Send packet through relay
    pub async fn send_to_peer(
        &mut self,
        peer_key: &PublicKey,
        packet: &[u8],
    ) -> Result<(), Error> {
        let frame = RelayFrame {
            frame_type: RelayFrameType::SendPacket,
            destination: Some(*peer_key),
            payload: packet.to_vec(),
        };
        
        self.conn.write_all(&frame.encode()).await?;
        Ok(())
    }
    
    /// Receive packet from relay
    pub async fn recv(&mut self) -> Result<(PublicKey, Vec<u8>), Error> {
        let frame = RelayFrame::decode(&mut self.conn).await?;
        
        match frame.frame_type {
            RelayFrameType::RecvPacket => {
                let source = frame.source.ok_or(Error::InvalidFrame)?;
                Ok((source, frame.payload))
            }
            _ => Err(Error::UnexpectedFrame),
        }
    }
}
```

---

## 12. Discovery Protocol

### 12.1 Privacy-Enhanced DHT

```rust
/// Privacy-preserving DHT for peer discovery
pub struct PrivateDht {
    /// Kademlia routing table
    routing_table: RoutingTable,
    
    /// Group keys for announcement encryption
    group_keys: HashMap<GroupId, GroupKey>,
    
    /// Our node ID
    node_id: NodeId,
}

impl PrivateDht {
    /// Announce presence (encrypted for group members only)
    pub async fn announce(
        &self,
        group_id: &GroupId,
        endpoints: &[Endpoint],
    ) -> Result<(), Error> {
        let group_key = self.group_keys.get(group_id)
            .ok_or(Error::UnknownGroup)?;
        
        // Derive DHT key (only group members can compute)
        let dht_key = blake3::derive_key(
            "wraith-dht-announce-v2",
            &[group_key.as_bytes(), self.node_id.as_bytes()].concat(),
        );
        
        // Create announcement
        let announcement = Announcement {
            endpoints: endpoints.to_vec(),
            timestamp: SystemTime::now(),
            capabilities: self.capabilities(),
        };
        
        // Sign then encrypt
        let signed = self.sign_announcement(&announcement)?;
        let encrypted = self.encrypt_for_group(group_key, &signed)?;
        
        // Store in DHT
        self.store(&dht_key[..20], &encrypted).await
    }
    
    /// Lookup peer
    pub async fn lookup(
        &self,
        group_id: &GroupId,
        peer_id: &PeerId,
    ) -> Result<Vec<Endpoint>, Error> {
        let group_key = self.group_keys.get(group_id)
            .ok_or(Error::UnknownGroup)?;
        
        // Derive DHT key for this peer
        let dht_key = blake3::derive_key(
            "wraith-dht-announce-v2",
            &[group_key.as_bytes(), peer_id.as_bytes()].concat(),
        );
        
        // Retrieve from DHT
        let encrypted = self.get(&dht_key[..20]).await?;
        
        // Decrypt and verify
        let signed = self.decrypt_from_group(group_key, &encrypted)?;
        let announcement = self.verify_announcement(&signed, peer_id)?;
        
        Ok(announcement.endpoints)
    }
}
```

---

## 13. Group Communication

### 13.1 Group Modes

```rust
/// Group communication topology
pub enum GroupTopology {
    /// Hub and spoke (one coordinator)
    Centralized {
        coordinator: PeerId,
    },
    
    /// Full mesh (everyone connects to everyone)
    FullMesh,
    
    /// Tree-based distribution
    Tree {
        fanout: usize,
    },
    
    /// Gossip protocol
    Gossip {
        fanout: usize,
        rounds: usize,
    },
}

/// Group session
pub struct GroupSession {
    /// Group identifier
    pub group_id: GroupId,
    
    /// Group symmetric key
    pub group_key: GroupKey,
    
    /// Member list with roles
    pub members: HashMap<PeerId, MemberInfo>,
    
    /// Topology mode
    pub topology: GroupTopology,
    
    /// TreeKEM state for forward secrecy
    pub tree_kem: Option<TreeKem>,
    
    /// Pairwise sessions
    pub peer_sessions: HashMap<PeerId, Session>,
}
```

### 13.2 TreeKEM for Group Forward Secrecy

```rust
/// TreeKEM for scalable group key agreement
pub struct TreeKem {
    /// Binary tree of nodes
    nodes: Vec<TreeNode>,
    
    /// Our position in tree
    position: usize,
    
    /// Our leaf secret
    leaf_secret: Zeroizing<[u8; 32]>,
}

impl TreeKem {
    /// Update our key and broadcast
    pub fn self_update(&mut self) -> KeyUpdate {
        // Generate new leaf secret
        self.leaf_secret = Zeroizing::new(OsRng.gen());
        
        // Compute path secrets up to root
        let mut path_secrets = Vec::new();
        let mut current = self.position;
        
        while current != 0 {
            let parent = (current - 1) / 2;
            let sibling = if current % 2 == 1 { current + 1 } else { current - 1 };
            
            // Encrypt to sibling's public key
            let path_secret = self.derive_path_secret(current);
            let encrypted = self.encrypt_to_node(sibling, &path_secret);
            
            path_secrets.push((parent, encrypted));
            current = parent;
        }
        
        KeyUpdate {
            sender: self.position,
            path_secrets,
        }
    }
    
    /// Process key update from another member
    pub fn process_update(&mut self, update: &KeyUpdate) -> Result<(), Error> {
        // Find the path secret we can decrypt
        for (node, encrypted) in &update.path_secrets {
            if self.can_decrypt(*node) {
                let path_secret = self.decrypt(encrypted)?;
                self.update_path(*node, &path_secret);
                return Ok(());
            }
        }
        
        Err(Error::CannotDecrypt)
    }
    
    /// Derive current group key
    pub fn group_key(&self) -> GroupKey {
        GroupKey(self.nodes[0].secret.clone())
    }
}
```

---

## 14. Real-Time Extensions

### 14.1 QoS Modes

```rust
/// Quality of Service configuration
pub struct QosConfig {
    /// Mode for this stream
    pub mode: QosMode,
    
    /// Target latency
    pub target_latency: Duration,
    
    /// Jitter buffer size
    pub jitter_buffer: Duration,
    
    /// FEC configuration
    pub fec: Option<FecConfig>,
}

#[derive(Clone, Copy, Debug)]
pub enum QosMode {
    /// Reliable ordered (file transfer)
    Reliable,
    
    /// Unreliable ordered (live video)
    UnreliableOrdered { max_age: Duration },
    
    /// Unreliable unordered (game state)
    UnreliableUnordered,
    
    /// Partial reliability (VoIP)
    PartiallyReliable { max_retransmits: u8 },
}
```

### 14.2 Forward Error Correction

```rust
/// FEC configuration
pub struct FecConfig {
    /// FEC algorithm
    pub algorithm: FecAlgorithm,
    
    /// Redundancy ratio
    pub redundancy: f32,
    
    /// Block size
    pub block_size: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum FecAlgorithm {
    /// Reed-Solomon
    ReedSolomon,
    
    /// LDPC
    Ldpc,
    
    /// Simple XOR
    Xor,
}

/// FEC encoder
pub struct FecEncoder {
    config: FecConfig,
    block_buffer: Vec<Vec<u8>>,
}

impl FecEncoder {
    /// Add packet to block
    pub fn add_packet(&mut self, packet: &[u8]) -> Option<Vec<Vec<u8>>> {
        self.block_buffer.push(packet.to_vec());
        
        if self.block_buffer.len() >= self.config.block_size {
            let repair = self.generate_repair();
            let block = std::mem::take(&mut self.block_buffer);
            Some([block, repair].concat())
        } else {
            None
        }
    }
    
    /// Generate repair packets
    fn generate_repair(&self) -> Vec<Vec<u8>> {
        match self.config.algorithm {
            FecAlgorithm::Xor => {
                // Simple XOR of all packets
                let mut repair = vec![0u8; self.max_packet_size()];
                for packet in &self.block_buffer {
                    for (i, byte) in packet.iter().enumerate() {
                        repair[i] ^= byte;
                    }
                }
                vec![repair]
            }
            // ... other algorithms
        }
    }
}
```

---

## 15. Error Handling

### 15.1 Error Codes

```rust
/// Protocol error codes
#[repr(u32)]
pub enum ErrorCode {
    // General errors (0x0000-0x00FF)
    NoError = 0x0000,
    InternalError = 0x0001,
    ProtocolError = 0x0002,
    
    // Cryptographic errors (0x0100-0x01FF)
    CryptoError = 0x0100,
    DecryptionFailed = 0x0101,
    AuthenticationFailed = 0x0102,
    RekeyFailed = 0x0103,
    
    // Flow control errors (0x0200-0x02FF)
    FlowControlError = 0x0200,
    WindowExceeded = 0x0201,
    
    // Stream errors (0x0300-0x03FF)
    StreamError = 0x0300,
    StreamLimitExceeded = 0x0301,
    InvalidStreamId = 0x0302,
    StreamClosed = 0x0303,
    
    // Connection errors (0x0400-0x04FF)
    ConnectionError = 0x0400,
    IdleTimeout = 0x0401,
    HandshakeTimeout = 0x0402,
    VersionMismatch = 0x0403,
    
    // Transport errors (0x0500-0x05FF)
    TransportError = 0x0500,
    MtuExceeded = 0x0501,
    
    // Application errors (0x1000-0xFFFF)
    ApplicationError = 0x1000,
}
```

### 15.2 Loss Detection

```rust
/// Loss detection configuration
pub struct LossDetection {
    /// Time threshold for loss
    pub time_threshold: Duration,
    
    /// Packet threshold for loss
    pub packet_threshold: u32,
    
    /// PTO calculation
    pub pto: Duration,
}

impl LossDetection {
    /// Detect lost packets
    pub fn detect_losses(
        &self,
        unacked: &BTreeMap<u32, SentPacket>,
        largest_acked: u32,
        now: Instant,
    ) -> Vec<u32> {
        let mut lost = Vec::new();
        
        for (&seq, packet) in unacked {
            // Time-based loss
            if now.duration_since(packet.sent_time) > self.time_threshold {
                lost.push(seq);
                continue;
            }
            
            // Packet-based loss
            if largest_acked.saturating_sub(seq) >= self.packet_threshold {
                lost.push(seq);
            }
        }
        
        lost
    }
    
    /// Calculate PTO
    pub fn calculate_pto(&self, srtt: Duration, rtt_var: Duration) -> Duration {
        srtt + (rtt_var * 4).max(Duration::from_millis(1)) + Duration::from_millis(25)
    }
}
```

---

## 16. Security Properties

### 16.1 Security Guarantees

| Property | Mechanism | Level |
|----------|-----------|-------|
| Confidentiality | XChaCha20-Poly1305 | 256-bit |
| Integrity | Poly1305 MAC | 128-bit |
| Authenticity | Noise_XX + Ed25519 | 128-bit |
| Forward Secrecy | Ephemeral DH + Ratchet | Per-packet |
| Post-Quantum | ML-KEM-768 Hybrid | 128-bit PQ |
| Replay Protection | Nonce + Window | Full |
| Traffic Analysis | Padding + Timing + Cover | Best-effort |
| Active Probing | Proof-of-knowledge | Full |

### 16.2 Side-Channel Mitigations

```rust
/// Side-channel mitigation configuration
pub struct SideChannelConfig {
    /// Use constant-time comparisons
    pub constant_time_compare: bool,
    
    /// Use memory-locked buffers for keys
    pub locked_memory: bool,
    
    /// Clear sensitive data after use
    pub zeroize_on_drop: bool,
    
    /// Avoid data-dependent branches
    pub constant_time_select: bool,
}

impl Default for SideChannelConfig {
    fn default() -> Self {
        Self {
            constant_time_compare: true,
            locked_memory: true,
            zeroize_on_drop: true,
            constant_time_select: true,
        }
    }
}
```

---

## 17. Protocol Constants

### 17.1 Timing Constants

```rust
pub mod timing {
    use std::time::Duration;
    
    /// Initial RTT estimate
    pub const INITIAL_RTT: Duration = Duration::from_millis(100);
    
    /// Maximum ACK delay
    pub const MAX_ACK_DELAY: Duration = Duration::from_millis(25);
    
    /// Idle timeout
    pub const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
    
    /// Handshake timeout
    pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
    
    /// DH ratchet interval
    pub const REKEY_INTERVAL: Duration = Duration::from_secs(120);
    
    /// Minimum cover traffic interval
    pub const MIN_COVER_INTERVAL: Duration = Duration::from_millis(100);
    
    /// ProbeRTT duration
    pub const PROBE_RTT_DURATION: Duration = Duration::from_millis(200);
    
    /// Clock skew tolerance for probing resistance
    pub const MAX_CLOCK_SKEW: Duration = Duration::from_secs(60);
}
```

### 17.2 Size Constants

```rust
pub mod sizes {
    /// Inner frame header size
    pub const FRAME_HEADER_SIZE: usize = 24;
    
    /// AEAD tag size
    pub const AUTH_TAG_SIZE: usize = 16;
    
    /// Connection ID size
    pub const CONNECTION_ID_SIZE: usize = 8;
    
    /// Minimum packet size
    pub const MIN_PACKET_SIZE: usize = 64;
    
    /// Default MTU
    pub const DEFAULT_MTU: usize = 1500;
    
    /// Maximum payload (default MTU)
    pub const MAX_PAYLOAD_DEFAULT: usize = 1428;
    
    /// Jumbo MTU
    pub const JUMBO_MTU: usize = 9000;
    
    /// Default chunk size
    pub const DEFAULT_CHUNK_SIZE: usize = 262144;  // 256 KiB
    
    /// Maximum streams per connection
    pub const MAX_STREAMS: u16 = 16384;
    
    /// Initial flow control window
    pub const INITIAL_WINDOW: u64 = 1048576;  // 1 MiB
    
    /// Maximum flow control window
    pub const MAX_WINDOW: u64 = 16777216;  // 16 MiB
    
    /// ML-KEM-768 public key size
    pub const ML_KEM_768_PK_SIZE: usize = 1184;
    
    /// ML-KEM-768 ciphertext size
    pub const ML_KEM_768_CT_SIZE: usize = 1088;
}
```

### 17.3 Cryptographic Constants

```rust
pub mod crypto {
    /// X25519 key sizes
    pub const X25519_PUBLIC_KEY_SIZE: usize = 32;
    pub const X25519_SECRET_KEY_SIZE: usize = 32;
    
    /// Elligator2 representative size
    pub const ELLIGATOR_REPR_SIZE: usize = 32;
    
    /// XChaCha20-Poly1305 sizes
    pub const XCHACHA_KEY_SIZE: usize = 32;
    pub const XCHACHA_NONCE_SIZE: usize = 24;
    
    /// Protocol nonce size
    pub const PROTOCOL_NONCE_SIZE: usize = 8;
    
    /// BLAKE3 output size
    pub const BLAKE3_OUTPUT_SIZE: usize = 32;
    
    /// Chunk hash size (truncated)
    pub const CHUNK_HASH_SIZE: usize = 16;
    
    /// Ed25519 signature size
    pub const ED25519_SIGNATURE_SIZE: usize = 64;
    
    /// Stateless reset token size
    pub const RESET_TOKEN_SIZE: usize = 16;
    
    /// Proof-of-knowledge size
    pub const PROOF_SIZE: usize = 32;
}
```

---

## 18. Appendices

### Appendix A: Wire Format Quick Reference

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        OUTER PACKET (Polymorphic)                       │
├─────────────────────────────────────────────────────────────────────────┤
│  Format derived from session secret                                     │
│  Fields: CID, Payload, Tag (order varies)                              │
│  Optional: Dummy fields, Length fields, Version bytes                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                        INNER FRAME (Fixed)                              │
├─────────────────────────────────────────────────────────────────────────┤
│  [Nonce:8][Type:1][Flags:1][StreamID:2][Seq:4][Offset:8]               │
│  [PayloadLen:2][ExtCount:1][Extensions...][Payload...][Padding...]     │
│  Header: 24 bytes minimum                                               │
└─────────────────────────────────────────────────────────────────────────┘
```

### Appendix B: Cryptographic Agility Matrix

| Suite | Classical KEM | PQ KEM | AEAD | Hash | Signature |
|-------|--------------|--------|------|------|-----------|
| A (Default) | X25519 | ML-KEM-768 | XChaCha20-Poly1305 | BLAKE3 | Ed25519 |
| B (HW Accel) | X25519 | ML-KEM-768 | AES-256-GCM | SHA-256 | Ed25519 |
| C (Max Sec) | X448 | ML-KEM-1024 | XChaCha20-Poly1305 | BLAKE3 | Ed448 |
| D (Classical) | X25519 | None | XChaCha20-Poly1305 | BLAKE3 | Ed25519 |

### Appendix C: Resource Profile Reference

| Profile | CPU | Memory | Bandwidth | Features |
|---------|-----|--------|-----------|----------|
| Performance | Unlimited | 256MB+ | Unlimited | All |
| Balanced | 50% | 64MB | 1 Gbps | Most |
| Constrained | 10% | 16MB | 10 Mbps | Core only |
| Metered | Any | 32MB | Budget-limited | No cover traffic |
| Stealth | Any | 64MB | Shaped | Full obfuscation |

---

## Document Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0-DRAFT | 2025-11 | Initial specification |
| 2.0.0 | 2026-01 | Post-quantum crypto, polymorphic wire format, transport abstraction, group support, real-time extensions, enhanced obfuscation |

---

*End of WRAITH Protocol v2 Technical Specification*
