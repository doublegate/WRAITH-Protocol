# WRAITH Protocol v2 Wire Format Changes

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Planning Document
**Authors:** WRAITH Protocol Team

---

## Table of Contents

1. [Overview](#overview)
2. [Frame Header Changes](#frame-header-changes)
3. [Connection ID Expansion](#connection-id-expansion)
4. [Polymorphic Wire Format](#polymorphic-wire-format)
5. [Frame Types](#frame-types)
6. [Packet Structure](#packet-structure)
7. [Encoding Examples](#encoding-examples)
8. [Migration Considerations](#migration-considerations)

---

## Overview

WRAITH Protocol v2 introduces significant wire format changes to enhance security, support post-quantum cryptography, and enable traffic analysis resistance through polymorphic encoding.

### Key Changes Summary

| Aspect | v1 | v2 | Rationale |
|--------|----|----|-----------|
| Header Size | 20 bytes | 24 bytes | 128-bit CID, larger sequence |
| Connection ID | 64-bit | 128-bit | Collision resistance, PQ future |
| Sequence Number | 32-bit | 64-bit | Long-lived sessions |
| Version Field | 4 bits | 8 bits | Extended version space |
| Format | Static | Polymorphic | Traffic analysis resistance |
| Padding | Fixed classes | Continuous | Better obfuscation |

---

## Frame Header Changes

### v1 Frame Header (20 bytes)

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|Version|  Type |     Flags     |           Reserved            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Sequence Number                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Length              |           Reserved            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                       Connection ID (64-bit)                  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

Total: 20 bytes
```

### v2 Frame Header (24 bytes)

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    Version    |     Type      |     Flags     |   Reserved    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                    Sequence Number (64-bit)                   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         Payload Length                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                                                               |
|                    Connection ID (128-bit)                    |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

Total: 24 bytes (28 bytes with 4-byte alignment padding)
```

### Field-by-Field Comparison

#### Version Field

```rust
// v1: 4-bit version (values 0-15)
pub struct V1Version(u4);  // Embedded in first byte with Type

// v2: 8-bit version (values 0-255)
pub struct V2Version(u8);

impl V2Version {
    pub const V2_0: Self = Self(0x20);  // 2.0
    pub const V2_1: Self = Self(0x21);  // 2.1

    // Version 0x01-0x0F reserved for v1 compatibility
    pub fn is_v1_compat(&self) -> bool {
        self.0 <= 0x0F
    }
}
```

#### Frame Type Field

```rust
// v1: 4-bit frame type (16 types max)
#[repr(u8)]
pub enum V1FrameType {
    Data = 0,
    Ack = 1,
    Control = 2,
    Rekey = 3,
    Ping = 4,
    Pong = 5,
    Close = 6,
    Pad = 7,
    // 8-15 reserved
}

// v2: 8-bit frame type (256 types)
#[repr(u8)]
pub enum V2FrameType {
    // Core types (0x00-0x0F)
    Data = 0x00,
    Ack = 0x01,
    Control = 0x02,
    Rekey = 0x03,
    Ping = 0x04,
    Pong = 0x05,
    Close = 0x06,
    Pad = 0x07,

    // Stream types (0x10-0x1F)
    StreamOpen = 0x10,
    StreamData = 0x11,
    StreamClose = 0x12,
    StreamReset = 0x13,
    StreamWindowUpdate = 0x14,
    StreamPriority = 0x15,

    // Path types (0x20-0x2F)
    PathProbe = 0x20,
    PathResponse = 0x21,
    PathMigrate = 0x22,
    PathAbandon = 0x23,

    // Group types (0x30-0x3F)
    GroupKeyUpdate = 0x30,
    GroupMemberAdd = 0x31,
    GroupMemberRemove = 0x32,

    // QoS types (0x40-0x4F)
    QoSRequest = 0x40,
    QoSGrant = 0x41,
    QoSRevoke = 0x42,

    // FEC types (0x50-0x5F)
    FecRepair = 0x50,
    FecAck = 0x51,

    // Reserved (0x60-0xFF)
}
```

#### Flags Field

```rust
// v1 Flags (8 bits)
bitflags! {
    pub struct V1Flags: u8 {
        const ENCRYPTED = 0b0000_0001;
        const COMPRESSED = 0b0000_0010;
        const PRIORITY = 0b0000_0100;
        const FIN = 0b0000_1000;
        // 4 bits unused
    }
}

// v2 Flags (8 bits, more defined)
bitflags! {
    pub struct V2Flags: u8 {
        const ENCRYPTED = 0b0000_0001;
        const COMPRESSED = 0b0000_0010;
        const PRIORITY_HIGH = 0b0000_0100;
        const PRIORITY_LOW = 0b0000_1000;
        const FIN = 0b0001_0000;
        const KEY_PHASE = 0b0010_0000;  // For key rotation
        const PATH_ID = 0b0100_0000;    // Multi-path indicator
        const RESERVED = 0b1000_0000;
    }
}
```

---

## Connection ID Expansion

### Rationale for 128-bit CIDs

1. **Collision Resistance**: 64-bit CIDs have birthday collision at ~2^32 connections
2. **Post-Quantum Safety**: Larger CID space for future PQ requirements
3. **Migration Support**: More bits for embedding migration hints
4. **Privacy**: Harder to track across networks

### Connection ID Structure

```rust
/// v2 Connection ID (128-bit)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId {
    bytes: [u8; 16],
}

impl ConnectionId {
    /// Generate a new random CID
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16];
        getrandom::getrandom(&mut bytes).expect("RNG failure");
        Self { bytes }
    }

    /// Generate with embedded metadata
    pub fn generate_with_meta(meta: CidMetadata) -> Self {
        let mut bytes = [0u8; 16];
        getrandom::getrandom(&mut bytes[4..]).expect("RNG failure");

        // Embed metadata in first 4 bytes
        bytes[0] = meta.version;
        bytes[1] = meta.flags;
        bytes[2..4].copy_from_slice(&meta.sequence.to_be_bytes());

        Self { bytes }
    }

    /// Migrate from v1 CID
    pub fn from_v1(v1_cid: u64) -> Self {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&v1_cid.to_be_bytes());
        bytes[8..12].copy_from_slice(b"MIG1");  // Migration marker
        getrandom::getrandom(&mut bytes[12..]).expect("RNG failure");
        Self { bytes }
    }

    /// Check if this CID is migrated from v1
    pub fn is_migrated_v1(&self) -> bool {
        &self.bytes[8..12] == b"MIG1"
    }

    /// Extract original v1 CID if migrated
    pub fn original_v1(&self) -> Option<u64> {
        if self.is_migrated_v1() {
            Some(u64::from_be_bytes(self.bytes[..8].try_into().unwrap()))
        } else {
            None
        }
    }
}

/// CID metadata embedded in first 4 bytes
#[derive(Clone, Copy)]
pub struct CidMetadata {
    pub version: u8,
    pub flags: u8,
    pub sequence: u16,
}
```

### CID Format Options

```
Standard CID (Random):
┌────────────────────────────────────────────────────────────────┐
│                     Random bytes (128 bits)                    │
└────────────────────────────────────────────────────────────────┘

CID with Metadata:
┌─────────┬─────────┬───────────┬────────────────────────────────┐
│ Version │  Flags  │  Sequence │      Random bytes (96 bits)    │
│ (8 bits)│ (8 bits)│ (16 bits) │                                │
└─────────┴─────────┴───────────┴────────────────────────────────┘

Migrated v1 CID:
┌────────────────────────────────┬──────────┬────────────────────┐
│      Original v1 CID (64 bits) │  "MIG1"  │  Random (32 bits)  │
└────────────────────────────────┴──────────┴────────────────────┘
```

---

## Polymorphic Wire Format

### Overview

The polymorphic wire format makes packet headers appear different for each session, preventing fingerprinting and traffic analysis.

### Format Derivation

```rust
/// Polymorphic format derived from session secret
pub struct PolymorphicFormat {
    /// Key for format transformation
    format_key: [u8; 32],

    /// Field byte positions (permuted)
    field_positions: FieldPositions,

    /// XOR mask for obfuscation
    xor_mask: [u8; 32],

    /// Byte swapping pattern
    swap_pattern: SwapPattern,
}

impl PolymorphicFormat {
    pub fn derive(session_secret: &[u8; 32]) -> Self {
        // Derive format key
        let format_key = blake3::keyed_hash(
            b"wraith-v2-format-derive-key-0001",
            session_secret,
        ).as_bytes().clone();

        // Derive field positions
        let pos_seed = blake3::keyed_hash(
            b"wraith-v2-format-positions-0001",
            session_secret,
        );
        let field_positions = FieldPositions::from_seed(pos_seed.as_bytes());

        // Derive XOR mask
        let xor_mask = blake3::keyed_hash(
            b"wraith-v2-format-xor-mask-0001",
            session_secret,
        ).as_bytes().clone();

        // Derive swap pattern
        let swap_seed = blake3::keyed_hash(
            b"wraith-v2-format-swap-ptrn-0001",
            session_secret,
        );
        let swap_pattern = SwapPattern::from_seed(swap_seed.as_bytes());

        Self {
            format_key,
            field_positions,
            xor_mask,
            swap_pattern,
        }
    }
}

/// Field positions within the header
#[derive(Clone)]
pub struct FieldPositions {
    version: (usize, usize),      // (start, length)
    frame_type: (usize, usize),
    flags: (usize, usize),
    sequence: (usize, usize),
    length: (usize, usize),
    connection_id: (usize, usize),
}

impl FieldPositions {
    pub fn from_seed(seed: &[u8]) -> Self {
        // Generate permutation from seed
        let mut positions: Vec<usize> = (0..24).collect();
        let mut rng = ChaCha20Rng::from_seed(seed[..32].try_into().unwrap());
        positions.shuffle(&mut rng);

        // Assign fields to position ranges
        Self {
            version: (positions[0], 1),
            frame_type: (positions[1], 1),
            flags: (positions[2], 1),
            sequence: Self::find_contiguous(&positions, 3, 8),
            length: Self::find_contiguous(&positions, 11, 4),
            connection_id: Self::find_contiguous(&positions, 15, 16),
        }
    }

    fn find_contiguous(positions: &[usize], start: usize, len: usize) -> (usize, usize) {
        // Find contiguous range starting at positions[start]
        let base = positions[start];
        (base, len)
    }
}
```

### Encoding Process

```rust
impl PolymorphicFormat {
    /// Encode a frame header using polymorphic format
    pub fn encode_header(&self, header: &FrameHeader) -> [u8; 24] {
        let mut encoded = [0u8; 24];

        // Step 1: Place fields at derived positions
        self.place_field(&mut encoded, &self.field_positions.version,
                         &[header.version]);
        self.place_field(&mut encoded, &self.field_positions.frame_type,
                         &[header.frame_type as u8]);
        self.place_field(&mut encoded, &self.field_positions.flags,
                         &[header.flags.bits()]);
        self.place_field(&mut encoded, &self.field_positions.sequence,
                         &header.sequence.to_be_bytes());
        self.place_field(&mut encoded, &self.field_positions.length,
                         &header.length.to_be_bytes());
        self.place_field(&mut encoded, &self.field_positions.connection_id,
                         &header.connection_id.as_bytes());

        // Step 2: Apply XOR mask
        for i in 0..24 {
            encoded[i] ^= self.xor_mask[i];
        }

        // Step 3: Apply byte swapping
        self.swap_pattern.apply(&mut encoded);

        encoded
    }

    /// Decode a polymorphic header
    pub fn decode_header(&self, data: &[u8; 24]) -> Result<FrameHeader, DecodeError> {
        let mut decoded = *data;

        // Step 1: Reverse byte swapping
        self.swap_pattern.reverse(&mut decoded);

        // Step 2: Remove XOR mask
        for i in 0..24 {
            decoded[i] ^= self.xor_mask[i];
        }

        // Step 3: Extract fields from derived positions
        let version = self.extract_u8(&decoded, &self.field_positions.version);
        let frame_type = FrameType::try_from(
            self.extract_u8(&decoded, &self.field_positions.frame_type)
        )?;
        let flags = Flags::from_bits(
            self.extract_u8(&decoded, &self.field_positions.flags)
        ).ok_or(DecodeError::InvalidFlags)?;
        let sequence = self.extract_u64(&decoded, &self.field_positions.sequence);
        let length = self.extract_u32(&decoded, &self.field_positions.length);
        let cid_bytes = self.extract_bytes(&decoded, &self.field_positions.connection_id);
        let connection_id = ConnectionId::from_bytes(cid_bytes.try_into()?);

        Ok(FrameHeader {
            version,
            frame_type,
            flags,
            sequence,
            length,
            connection_id,
        })
    }

    fn place_field(&self, dst: &mut [u8], pos: &(usize, usize), data: &[u8]) {
        let (start, len) = *pos;
        dst[start..start + len].copy_from_slice(&data[..len]);
    }

    fn extract_u8(&self, src: &[u8], pos: &(usize, usize)) -> u8 {
        src[pos.0]
    }

    fn extract_u64(&self, src: &[u8], pos: &(usize, usize)) -> u64 {
        let bytes: [u8; 8] = src[pos.0..pos.0 + 8].try_into().unwrap();
        u64::from_be_bytes(bytes)
    }

    fn extract_u32(&self, src: &[u8], pos: &(usize, usize)) -> u32 {
        let bytes: [u8; 4] = src[pos.0..pos.0 + 4].try_into().unwrap();
        u32::from_be_bytes(bytes)
    }
}
```

### Visual Example

```
Standard Header (before polymorphic encoding):
┌──────┬──────┬──────┬──────┬─────────────────┬─────────┬─────────────────┐
│ Ver  │ Type │ Flag │ Res  │   Sequence(64)  │Len(32)  │   CID (128)     │
│ [0]  │ [1]  │ [2]  │ [3]  │    [4-11]       │[12-15]  │   [16-31]       │
└──────┴──────┴──────┴──────┴─────────────────┴─────────┴─────────────────┘

After Polymorphic Encoding (example session):
┌──────┬──────┬──────┬──────┬─────────────────┬─────────┬─────────────────┐
│CID[3]│Seq[2]│ Flag │CID[7]│   CID[0-5]+Ver  │Type+Res │   Seq+Len+CID   │
│      │      │      │      │                 │         │                 │
└──────┴──────┴──────┴──────┴─────────────────┴─────────┴─────────────────┘
       │                            │
       └──── Fields are permuted ───┘

After XOR Mask:
┌─────────────────────────────────────────────────────────────────────────┐
│  All bytes XORed with session-derived mask - appears random             │
└─────────────────────────────────────────────────────────────────────────┘

After Byte Swapping:
┌─────────────────────────────────────────────────────────────────────────┐
│  Specific byte pairs swapped according to pattern - further obfuscation │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Frame Types

### New Frame Types in v2

```rust
/// Complete v2 frame type enumeration
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameType {
    // === Core Types (0x00-0x0F) - Same as v1 ===
    Data = 0x00,
    Ack = 0x01,
    Control = 0x02,
    Rekey = 0x03,
    Ping = 0x04,
    Pong = 0x05,
    Close = 0x06,
    Pad = 0x07,

    // === Stream Types (0x10-0x1F) - New in v2 ===
    /// Open a new stream
    StreamOpen = 0x10,
    /// Stream data (multiplexed)
    StreamData = 0x11,
    /// Graceful stream close
    StreamClose = 0x12,
    /// Abrupt stream reset
    StreamReset = 0x13,
    /// Flow control window update
    StreamWindowUpdate = 0x14,
    /// Stream priority change
    StreamPriority = 0x15,
    /// Stream blocked notification
    StreamBlocked = 0x16,

    // === Path Types (0x20-0x2F) - New in v2 ===
    /// Probe new network path
    PathProbe = 0x20,
    /// Response to path probe
    PathResponse = 0x21,
    /// Migrate to new path
    PathMigrate = 0x22,
    /// Abandon path
    PathAbandon = 0x23,
    /// Path MTU discovery
    PathMtuProbe = 0x24,
    /// Path challenge (keep-alive)
    PathChallenge = 0x25,

    // === Group Types (0x30-0x3F) - New in v2 ===
    /// Update group encryption key (TreeKEM)
    GroupKeyUpdate = 0x30,
    /// Add member to group
    GroupMemberAdd = 0x31,
    /// Remove member from group
    GroupMemberRemove = 0x32,
    /// Group message
    GroupMessage = 0x33,
    /// Group welcome (for new members)
    GroupWelcome = 0x34,

    // === QoS Types (0x40-0x4F) - New in v2 ===
    /// Request QoS parameters
    QoSRequest = 0x40,
    /// Grant QoS parameters
    QoSGrant = 0x41,
    /// Revoke QoS grant
    QoSRevoke = 0x42,
    /// QoS feedback
    QoSFeedback = 0x43,

    // === FEC Types (0x50-0x5F) - New in v2 ===
    /// FEC repair packet
    FecRepair = 0x50,
    /// FEC acknowledgment
    FecAck = 0x51,
    /// FEC parameter negotiation
    FecNegotiate = 0x52,

    // === Handshake Types (0x60-0x6F) - New in v2 ===
    /// Initial handshake message
    HandshakeInit = 0x60,
    /// Handshake response
    HandshakeResponse = 0x61,
    /// Handshake completion
    HandshakeComplete = 0x62,
    /// Post-quantum encapsulation
    HandshakePQEncap = 0x63,
    /// Version negotiation
    HandshakeVersion = 0x64,
}
```

### Frame Type Payloads

```rust
/// StreamOpen payload
#[derive(Clone, Debug)]
pub struct StreamOpenPayload {
    pub stream_id: u64,
    pub initial_window: u32,
    pub priority: u8,
    pub bidirectional: bool,
}

/// PathProbe payload
#[derive(Clone, Debug)]
pub struct PathProbePayload {
    pub path_id: u32,
    pub challenge: [u8; 8],
    pub source_cid: ConnectionId,
    pub dest_cid: ConnectionId,
}

/// GroupKeyUpdate payload (TreeKEM)
#[derive(Clone, Debug)]
pub struct GroupKeyUpdatePayload {
    pub epoch: u64,
    pub sender_index: u32,
    pub path_secret: Vec<u8>,
    pub tree_update: TreeUpdate,
}

/// FecRepair payload
#[derive(Clone, Debug)]
pub struct FecRepairPayload {
    pub block_id: u64,
    pub symbol_id: u16,
    pub repair_data: Vec<u8>,
}
```

---

## Packet Structure

### Complete Packet Layout

```
WRAITH v2 Packet Structure:
┌───────────────────────────────────────────────────────────────┐
│                      Outer Packet                             │
├───────────────────────────────────────────────────────────────┤
│ Connection ID Hint (8 bytes) - For routing, not auth         │
├───────────────────────────────────────────────────────────────┤
│                                                               │
│                   Encrypted Payload                           │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              Polymorphic Frame Header (24 bytes)        │  │
│  ├─────────────────────────────────────────────────────────┤  │
│  │                                                         │  │
│  │                    Frame Payload                        │  │
│  │                    (variable)                           │  │
│  │                                                         │  │
│  ├─────────────────────────────────────────────────────────┤  │
│  │                  Random Padding                         │  │
│  │              (continuous distribution)                  │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                               │
├───────────────────────────────────────────────────────────────┤
│              Authentication Tag (16 bytes)                    │
└───────────────────────────────────────────────────────────────┘

Minimum Packet Size: 8 + 24 + 0 + 0 + 16 = 48 bytes
Maximum Packet Size: 8 + 24 + 65535 + 1024 + 16 = 66607 bytes (typically MTU-limited)
```

### Encryption Scope

```rust
/// What gets encrypted vs what's in cleartext
pub struct PacketLayout {
    // Cleartext (for routing)
    pub cid_hint: [u8; 8],  // First 8 bytes of CID

    // Encrypted (AEAD)
    pub encrypted_frame: Vec<u8>,  // Header + Payload + Padding

    // Authentication
    pub auth_tag: [u8; 16],  // XChaCha20-Poly1305 tag
}

impl PacketLayout {
    pub fn encrypt(
        frame: &Frame,
        session: &Session,
        padding_strategy: &dyn PaddingStrategy,
    ) -> Result<Self> {
        // Build plaintext
        let header = session.format().encode_header(&frame.header);
        let padding_len = padding_strategy.sample(frame.payload.len());
        let mut plaintext = Vec::with_capacity(
            header.len() + frame.payload.len() + padding_len
        );
        plaintext.extend_from_slice(&header);
        plaintext.extend_from_slice(&frame.payload);
        plaintext.resize(plaintext.len() + padding_len, 0);
        getrandom::getrandom(&mut plaintext[header.len() + frame.payload.len()..])
            .expect("RNG failure");

        // Encrypt
        let (ciphertext, tag) = session.cipher().encrypt(
            &session.nonce_for_packet(frame.header.sequence),
            &plaintext,
            &frame.header.connection_id.as_bytes()[..8],  // AAD = CID hint
        )?;

        Ok(Self {
            cid_hint: frame.header.connection_id.as_bytes()[..8].try_into().unwrap(),
            encrypted_frame: ciphertext,
            auth_tag: tag,
        })
    }
}
```

---

## Encoding Examples

### Example 1: Data Frame Encoding

```rust
fn encode_data_frame_example() {
    // Create frame header
    let header = FrameHeader {
        version: 0x20,  // v2.0
        frame_type: FrameType::Data,
        flags: Flags::ENCRYPTED,
        sequence: 12345678901234,
        length: 1024,
        connection_id: ConnectionId::from_hex(
            "0102030405060708090a0b0c0d0e0f10"
        ),
    };

    // Derive polymorphic format
    let session_secret = [0x42u8; 32];  // Example
    let format = PolymorphicFormat::derive(&session_secret);

    // Encode header
    let encoded = format.encode_header(&header);

    // Resulting bytes (will look random to observer)
    println!("Encoded header: {:02x?}", encoded);
}
```

### Example 2: Stream Open Frame

```rust
fn encode_stream_open_example() {
    let header = FrameHeader {
        version: 0x20,
        frame_type: FrameType::StreamOpen,
        flags: Flags::empty(),
        sequence: 1,
        length: 14,  // StreamOpenPayload size
        connection_id: ConnectionId::generate(),
    };

    let payload = StreamOpenPayload {
        stream_id: 1,
        initial_window: 65536,
        priority: 128,
        bidirectional: true,
    };

    let frame = Frame {
        header,
        payload: payload.encode(),
    };

    // Full packet encoding with encryption
    let session = Session::new(/* ... */);
    let packet = PacketLayout::encrypt(&frame, &session, &DefaultPadding)?;
}
```

### Example 3: v1 to v2 Migration

```rust
fn migrate_v1_frame(v1_frame: &V1Frame) -> V2Frame {
    // Convert v1 header to v2
    let v2_header = FrameHeader {
        version: 0x20,  // Mark as v2
        frame_type: match v1_frame.header.frame_type {
            V1FrameType::Data => FrameType::Data,
            V1FrameType::Ack => FrameType::Ack,
            // ... other mappings
        },
        flags: convert_flags(v1_frame.header.flags),
        sequence: v1_frame.header.sequence as u64,  // Expand to 64-bit
        length: v1_frame.header.length as u32,      // Expand to 32-bit
        connection_id: ConnectionId::from_v1(v1_frame.header.connection_id),
    };

    V2Frame {
        header: v2_header,
        payload: v1_frame.payload.clone(),
    }
}
```

---

## Migration Considerations

### Wire Format Detection

```rust
/// Detect packet version from first bytes
pub fn detect_version(packet: &[u8]) -> Result<ProtocolVersion> {
    if packet.len() < 8 {
        return Err(Error::PacketTooShort);
    }

    // v1 packets have specific patterns
    // - Version nibble is 0x01
    // - Connection ID at fixed position

    // v2 packets are encrypted from the start
    // - Need to try decryption with known sessions
    // - Or use CID hint to find session

    // Heuristic: v1 has predictable structure
    let first_byte = packet[0];
    if (first_byte & 0xF0) == 0x10 {
        // Likely v1 (version nibble = 1)
        return Ok(ProtocolVersion::V1);
    }

    // Assume v2 (encrypted, no clear pattern)
    Ok(ProtocolVersion::V2)
}
```

### Compatibility Mode Encoding

```rust
/// Encode frame for v1 peer (compatibility mode)
pub fn encode_v1_compat(frame: &V2Frame) -> Result<Vec<u8>> {
    // Convert v2 header to v1 format
    let v1_header = V1Header {
        version_and_type: ((0x01 << 4) | (frame.header.frame_type as u8 & 0x0F)),
        flags: frame.header.flags.bits(),
        reserved: 0,
        sequence: frame.header.sequence as u32,
        length: frame.header.length as u16,
        connection_id: frame.header.connection_id.original_v1()
            .ok_or(Error::CidNotMigrated)?,
    };

    let mut encoded = Vec::with_capacity(20 + frame.payload.len());
    encoded.extend_from_slice(&v1_header.encode());
    encoded.extend_from_slice(&frame.payload);

    Ok(encoded)
}
```

### Gradual Migration Strategy

```
Phase 1: Dual-Format Support
┌─────────────────────────────────────────────────────────────┐
│ Server accepts both v1 and v2 packets                       │
│ Detection based on packet structure                         │
│ Sessions track negotiated version                           │
└─────────────────────────────────────────────────────────────┘

Phase 2: Prefer v2, Fallback v1
┌─────────────────────────────────────────────────────────────┐
│ New connections attempt v2 first                            │
│ Fallback to v1 if peer doesn't support v2                   │
│ Metrics track v1 vs v2 usage                                │
└─────────────────────────────────────────────────────────────┘

Phase 3: v2 Only (Deprecate v1)
┌─────────────────────────────────────────────────────────────┐
│ All new connections require v2                              │
│ Existing v1 sessions can complete                           │
│ No new v1 sessions accepted                                 │
└─────────────────────────────────────────────────────────────┘

Phase 4: Remove v1 Code
┌─────────────────────────────────────────────────────────────┐
│ v1 code paths removed                                       │
│ Only polymorphic v2 format                                  │
│ Reduced complexity and attack surface                       │
└─────────────────────────────────────────────────────────────┘
```

---

## Related Documents

- [Specification](01-WRAITH-Protocol-v2-Specification.md) - Complete protocol spec
- [Migration Guide](09-WRAITH-Protocol-v2-Migration-Guide.md) - Migration instructions
- [Crypto Upgrades](12-WRAITH-Protocol-v2-Crypto-Upgrades.md) - Cryptographic changes
- [Changelog](03-WRAITH-Protocol-v1-to-v2-Changelog.md) - Breaking changes

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial wire format changes document |
