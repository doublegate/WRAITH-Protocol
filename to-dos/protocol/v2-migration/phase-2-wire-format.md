# Phase 2: Wire Format

**Parent:** [v2 Migration Master Plan](../v2-migration-master.md)
**Version:** 1.0.0
**Status:** Planning (Post-v2.3.0)
**Story Points:** 75-95 SP
**Duration:** 2 weeks
**Dependencies:** Phase 1 (Crypto Foundation)

---

## Executive Summary

Phase 2 implements the expanded v2 wire format with 128-bit connection IDs, 64-bit sequence numbers, and polymorphic encoding. The polymorphic format uses session-derived field positions to resist traffic analysis fingerprinting.

### Objectives

1. Expand ConnectionId from 64-bit to 128-bit
2. Expand sequence numbers from 32-bit to 64-bit
3. Expand frame header from 20 to 24 bytes
4. Implement polymorphic field encoding
5. Maintain v1 compatibility mode

---

## Sprint Breakdown

### Sprint 2.1: ConnectionId Expansion (13-16 SP)

**Goal:** Migrate ConnectionId from 64-bit to 128-bit.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 2.1.1 | Define new `ConnectionId` struct (128-bit) | 2 | Critical | - |
| 2.1.2 | Implement `ConnectionId::generate()` | 2 | Critical | - |
| 2.1.3 | Implement serialization/deserialization | 2 | Critical | - |
| 2.1.4 | Add `from_v1()` migration helper | 2 | High | - |
| 2.1.5 | Add `is_migrated_v1()` detection | 1 | High | - |
| 2.1.6 | Update all CID usage sites | 5 | Critical | - |
| 2.1.7 | Deprecate 64-bit CID type | 1 | Medium | - |
| 2.1.8 | Unit tests (generation, serialization) | 2 | Critical | - |

**Acceptance Criteria:**
- [ ] ConnectionId is 128 bits (16 bytes)
- [ ] Cryptographically random generation
- [ ] v1 CIDs can be migrated (zero-extended)
- [ ] Migrated CIDs are detectable
- [ ] No CID collisions in test suite

**Wire Format Change:**
```
v1 Outer Packet:
┌─────────────────────────────────┐
│ Connection ID (8 bytes)         │
├─────────────────────────────────┤
│ Encrypted Payload               │
├─────────────────────────────────┤
│ Auth Tag (16 bytes)             │
└─────────────────────────────────┘

v2 Outer Packet:
┌─────────────────────────────────┐
│ Connection ID (16 bytes)        │
├─────────────────────────────────┤
│ Encrypted Payload               │
├─────────────────────────────────┤
│ Auth Tag (16 bytes)             │
└─────────────────────────────────┘
```

**Code Location:** `crates/wraith-core/src/frame/connection_id.rs`

---

### Sprint 2.2: Frame Header Expansion (18-22 SP)

**Goal:** Expand frame header to 24 bytes with larger fields.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 2.2.1 | Define new `FrameHeader` struct | 3 | Critical | - |
| 2.2.2 | Expand sequence to 64-bit | 2 | Critical | - |
| 2.2.3 | Expand length to 32-bit | 2 | Critical | - |
| 2.2.4 | Add version byte (full byte) | 1 | Critical | - |
| 2.2.5 | Implement type-safe `FrameType` enum | 2 | High | - |
| 2.2.6 | Implement `Flags` bitflags type | 2 | High | - |
| 2.2.7 | Header encoding (native byte order) | 3 | Critical | - |
| 2.2.8 | Header decoding with validation | 3 | Critical | - |
| 2.2.9 | Update all header usage sites | 5 | Critical | - |
| 2.2.10 | SIMD-optimized encoding/decoding | 3 | Medium | - |
| 2.2.11 | Unit tests (round-trip, edge cases) | 3 | Critical | - |

**Acceptance Criteria:**
- [ ] Header is exactly 24 bytes
- [ ] 64-bit sequences support long-lived sessions
- [ ] 32-bit length supports large frames (up to 4GB)
- [ ] FrameType is type-safe enum
- [ ] Encoding/decoding is correct
- [ ] SIMD optimization available on x86_64

**Header Layout (v2):**
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤
│   Version   │  Frame Type │     Flags     │    Reserved     │ 0-3
├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤
│                     Sequence Number (64-bit)                 │ 4-11
│                                                              │
├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤
│                     Payload Length (32-bit)                  │ 12-15
├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤
│                     Stream ID (32-bit)                       │ 16-19
├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤
│                     Reserved / Extension                     │ 20-23
└─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┘
```

**Code Location:** `crates/wraith-core/src/frame/header.rs`

---

### Sprint 2.3: Polymorphic Format (26-32 SP)

**Goal:** Implement session-derived polymorphic wire encoding.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 2.3.1 | Define `PolymorphicFormat` struct | 3 | Critical | - |
| 2.3.2 | Derive format key from session secret | 3 | Critical | - |
| 2.3.3 | Compute field positions from format key | 5 | Critical | - |
| 2.3.4 | Implement XOR masking of header bytes | 3 | Critical | - |
| 2.3.5 | Implement polymorphic `encode_header()` | 5 | Critical | - |
| 2.3.6 | Implement polymorphic `decode_header()` | 5 | Critical | - |
| 2.3.7 | Field position permutation table | 3 | High | - |
| 2.3.8 | Format validation (detect corruption) | 2 | High | - |
| 2.3.9 | Performance optimization (lookup tables) | 3 | Medium | - |
| 2.3.10 | Unit tests (encode/decode round-trip) | 3 | Critical | - |
| 2.3.11 | Property tests (position uniqueness) | 2 | High | - |

**Acceptance Criteria:**
- [ ] Field positions derived from session secret
- [ ] Same session always produces same format
- [ ] Different sessions produce different formats
- [ ] XOR mask makes headers appear random
- [ ] Decoding reverses encoding exactly
- [ ] No information leakage about format key

**Polymorphic Encoding Algorithm:**
```rust
impl PolymorphicFormat {
    pub fn derive(session_secret: &[u8; 32]) -> Self {
        // Derive format key
        let format_key = blake3::keyed_hash(
            b"wraith-v2-polymorphic-format-key",
            session_secret,
        );

        // Derive field positions (Fisher-Yates shuffle)
        let positions = Self::derive_positions(&format_key);

        // Derive XOR mask
        let xor_mask = blake3::keyed_hash(
            b"wraith-v2-polymorphic-xor-mask",
            session_secret,
        );

        Self {
            format_key: format_key.into(),
            field_positions: positions,
            xor_mask: xor_mask.into(),
        }
    }

    pub fn encode_header(&self, header: &FrameHeader) -> [u8; 24] {
        let mut encoded = [0u8; 24];

        // Place fields at derived positions
        self.write_field(&mut encoded, 0, &[header.version]);
        self.write_field(&mut encoded, 1, &[header.frame_type.into()]);
        self.write_field(&mut encoded, 2, &header.flags.bits().to_le_bytes());
        self.write_field(&mut encoded, 3, &header.sequence.to_le_bytes());
        self.write_field(&mut encoded, 4, &header.length.to_le_bytes());
        self.write_field(&mut encoded, 5, &header.stream_id.to_le_bytes());

        // Apply XOR mask
        for (i, byte) in encoded.iter_mut().enumerate() {
            *byte ^= self.xor_mask[i % 32];
        }

        encoded
    }
}
```

**Code Location:** `crates/wraith-core/src/frame/polymorphic.rs`

---

### Sprint 2.4: v1 Compatibility (10-14 SP)

**Goal:** Implement v1 wire format compatibility mode.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 2.4.1 | Define `WireFormat` enum (V1, V2, V2Polymorphic) | 2 | Critical | - |
| 2.4.2 | Implement v1 header encoding | 2 | High | - |
| 2.4.3 | Implement v1 header decoding | 2 | High | - |
| 2.4.4 | Format detection (v1 vs v2) | 3 | Critical | - |
| 2.4.5 | Format negotiation during handshake | 3 | Critical | - |
| 2.4.6 | Compat mode configuration flag | 1 | High | - |
| 2.4.7 | Integration tests (v1 interop) | 3 | Critical | - |

**Acceptance Criteria:**
- [ ] v1 format encoding/decoding works
- [ ] Format auto-detected from packet
- [ ] Negotiation selects best common format
- [ ] Compat mode can be disabled
- [ ] v1 clients can connect to v2 servers

**Code Location:** `crates/wraith-core/src/frame/compat.rs`

---

### Sprint 2.5: Frame Types & Flags (8-11 SP)

**Goal:** Update frame types and flags for v2.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 2.5.1 | Define extended `FrameType` enum | 2 | Critical | - |
| 2.5.2 | Add v2 frame types (STREAM_*, PATH_*) | 2 | Critical | - |
| 2.5.3 | Define `Flags` bitflags | 2 | High | - |
| 2.5.4 | Frame type validation | 1 | High | - |
| 2.5.5 | Unknown frame type handling | 1 | High | - |
| 2.5.6 | Unit tests (all frame types) | 2 | Critical | - |

**v2 Frame Types:**
```rust
pub enum FrameType {
    // Data frames (0x00-0x0F)
    Data = 0x00,
    DataFin = 0x01,

    // Control frames (0x10-0x1F)
    Ack = 0x10,
    AckEcn = 0x11,
    Ping = 0x12,
    Pong = 0x13,

    // Crypto frames (0x20-0x2F)
    Rekey = 0x20,
    RekeyAck = 0x21,

    // Stream frames (0x30-0x3F)
    StreamOpen = 0x30,
    StreamData = 0x31,
    StreamClose = 0x32,
    StreamReset = 0x33,
    StreamWindow = 0x34,

    // Path frames (0x40-0x4F)
    PathChallenge = 0x40,
    PathResponse = 0x41,
    PathMigrate = 0x42,

    // Session frames (0x50-0x5F)
    Close = 0x50,
    CloseAck = 0x51,

    // Obfuscation frames (0xF0-0xFF)
    Padding = 0xF0,
    PaddingRandom = 0xF1,
}
```

**Code Location:** `crates/wraith-core/src/frame/types.rs`

---

## Technical Specifications

### Wire Format Comparison

| Field | v1 Size | v2 Size | Change |
|-------|---------|---------|--------|
| Connection ID | 8 bytes | 16 bytes | +8 bytes |
| Version | 4 bits | 8 bits | +4 bits |
| Frame Type | 4 bits | 8 bits | +4 bits |
| Flags | 8 bits | 8 bits | Same |
| Sequence | 32 bits | 64 bits | +32 bits |
| Length | 16 bits | 32 bits | +16 bits |
| **Header Total** | **20 bytes** | **24 bytes** | **+4 bytes** |

### Polymorphic Format Parameters

| Parameter | Value |
|-----------|-------|
| Format Key Size | 32 bytes |
| XOR Mask Size | 32 bytes |
| Field Count | 6 |
| Position Permutations | 720 (6!) |
| Format Derivation | BLAKE3 keyed hash |

### Field Sizes for Position Calculation

| Field | Size | Position Options |
|-------|------|------------------|
| Version | 1 byte | Any |
| Frame Type | 1 byte | Any |
| Flags | 2 bytes | Any |
| Sequence | 8 bytes | Any |
| Length | 4 bytes | Any |
| Stream ID | 4 bytes | Any |
| Reserved | 4 bytes | Any |

---

## Testing Requirements

### Test Categories

| Category | Target Coverage | Method |
|----------|-----------------|--------|
| Unit Tests | 90% | Standard test framework |
| Property Tests | Encoding invariants | `proptest` |
| Compatibility | v1 interop | Cross-version tests |
| Fuzz Tests | Parser boundaries | `libfuzzer` |

### Test Cases

| Test Case | Description |
|-----------|-------------|
| T2.1 | ConnectionId generation uniqueness |
| T2.2 | Header encode/decode round-trip |
| T2.3 | Polymorphic format determinism |
| T2.4 | v1 compatibility encode/decode |
| T2.5 | Format detection accuracy |
| T2.6 | SIMD optimization correctness |
| T2.7 | Invalid header rejection |

---

## Dependencies

### Phase Dependencies

| Dependency | Type | Notes |
|------------|------|-------|
| Phase 1 | Required | Session secret for polymorphic derivation |

### Internal Crate Dependencies

| Crate | Dependency Type |
|-------|----------------|
| wraith-crypto | Key derivation for format |
| wraith-transport | Consumer of wire format |
| wraith-obfuscation | Consumer of wire format |

---

## Risk Assessment

### High Risk

| Risk | Mitigation |
|------|------------|
| Wire format incompatibility | Extensive v1 interop testing |
| Performance regression | SIMD optimization, benchmarks |
| Polymorphic format weakness | Cryptographic review |

### Medium Risk

| Risk | Mitigation |
|------|------------|
| Migration complexity | Clear migration path |
| Parser vulnerabilities | Fuzzing, bounds checking |

---

## Deliverables Checklist

### Code Deliverables

- [ ] `crates/wraith-core/src/frame/connection_id.rs` - 128-bit CID
- [ ] `crates/wraith-core/src/frame/header.rs` - 24-byte header
- [ ] `crates/wraith-core/src/frame/polymorphic.rs` - Polymorphic format
- [ ] `crates/wraith-core/src/frame/compat.rs` - v1 compatibility
- [ ] `crates/wraith-core/src/frame/types.rs` - Frame types

### Test Deliverables

- [ ] Unit tests for all modules
- [ ] Property tests for encoding
- [ ] v1 interoperability tests
- [ ] Fuzz tests for parser
- [ ] SIMD correctness tests

### Documentation Deliverables

- [ ] Wire format specification update
- [ ] Migration guide for wire format
- [ ] Rustdoc for all public APIs

---

## Gap Analysis (v2.3.7 Assessment)

### Current Implementation State

| Component | Status | Notes |
|-----------|--------|-------|
| ConnectionId | u64 (8 bytes) | `crates/wraith-core/src/session.rs:25` - `ConnectionId(u64)` |
| Frame Header | 28 bytes | `FRAME_HEADER_SIZE = 28` in `lib.rs:105` |
| Sequence Number | u32 | `FrameHeader.sequence: u32` in `frame.rs:195` |
| Payload Length | u16 | `FrameHeader.payload_len: u16` in `frame.rs:199` |
| Stream ID | u16 | `FrameHeader.stream_id: u16` in `frame.rs:193` |
| FrameType | 16 types (0x00-0x0F) | `frame.rs:39-74` - no GROUP/QOS/FEC/DATAGRAM types |
| SIMD parsing | COMPLETE | x86_64 SSE2 and ARM64 NEON in `frame.rs` |
| FrameBuilder | COMPLETE | Builder pattern in `frame.rs:656` |
| Zero-copy Frame | COMPLETE | `Frame<'a>` in `frame.rs:348` |

### Key Discrepancy

The v2 docs (doc 11) state v1 header is 20 bytes, but the actual v1 header is **28 bytes** with this layout:
- Nonce (8B) + FrameType (1B) + Flags (1B) + StreamID (2B) + Sequence (4B) + Offset (8B) + PayloadLen (2B) + Padding (2B)

The v2 spec header (doc 01, section 4.2) is 24 bytes:
- Nonce (8B) + FrameType (1B) + Flags (1B) + StreamID (2B) + Sequence (4B) + Offset (8B) + PayloadLen (2B) + ExtCount (1B)

This means the actual migration shrinks the header by 4 bytes (from 28 to 24), not expands it. The v2 spec adds an extension framework (ExtCount + variable Extensions) but the fixed header is smaller.

However, doc 11 also specifies a separate "outer" header format where the ConnectionId and polymorphic encoding apply at the packet level (not inner frame level). The inner frame format is post-decryption.

### Gaps Identified

1. **128-bit ConnectionId**: Current `ConnectionId(u64)` must expand to 128-bit. Affects: session.rs, node, all packet handling. ~60 usage sites found via grep. Estimated ~300 lines changed.

2. **Polymorphic wire format**: Entirely new. Session-derived field positions, XOR masking, byte swapping. Estimated ~800 lines new code.

3. **New frame types**: Missing 8+ v2 types: GROUP_JOIN/LEAVE/REKEY, QOS_UPDATE, FEC_REPAIR, PRIORITY, DATAGRAM, TIMESTAMP. Also need expanded type range (0x10-0x6F per doc 11). Estimated ~200 lines.

4. **Frame flags expansion**: v2 adds ECN, RTX, EXT, CMP bits. Current flags only define FIN, SYN, ACK, PRI. Estimated ~50 lines.

5. **Extension framework**: ExtCount + variable-length extensions per frame. Not present. Estimated ~400 lines.

6. **Version field**: v2 uses full byte for version (0x20 = v2.0). Current code has no explicit version in wire format. Estimated ~100 lines.

7. **Inner frame format reconciliation**: The v1 inner frame at 28 bytes already has Nonce+Offset fields. v2 inner frame at 24 bytes is actually *more compact*. Need to reconcile carefully.

### Inaccuracies in Current Plan

- Sprint 2.2 states "Expand frame header from 20 to 24 bytes" -- actual expansion is from 28-byte v1 inner frame. In fact it's a format change, not a simple expansion.
- Sprint 2.1 states CID in outer packet goes from 8 to 16 bytes -- this is correct for the outer packet format.
- Sprint 2.5 frame type numbering in TODO doesn't match doc 01 section 4.3 numbering. Doc 01 uses 0x10-0x17 for GROUP/QOS/FEC/PRIORITY/DATAGRAM/TIMESTAMP. Doc 11 uses different numbering (0x10-0x1F stream, 0x20-0x2F path, 0x30-0x3F group, etc.). Need to standardize on doc 11 numbering.

### Client Impact

All 12 clients depend on `wraith-core` for frame types. ConnectionId change affects:
- wraith-transfer (wraith-core)
- wraith-chat (wraith-core, wraith-crypto)
- wraith-android, wraith-ios (wraith-core, wraith-crypto)
- wraith-redops team-server/operator-client (wraith-core)
- wraith-recon, wraith-mesh, wraith-share, wraith-stream, wraith-publish, wraith-vault, wraith-sync

### Revised Story Point Estimate

| Sprint | Original SP | Revised SP | Notes |
|--------|------------|------------|-------|
| 2.1 ConnectionId | 13-16 | 16-20 | More usage sites than anticipated (~60) |
| 2.2 Frame Header | 18-22 | 15-18 | Header shrinks, less SIMD rework needed |
| 2.3 Polymorphic | 26-32 | 26-32 | Unchanged |
| 2.4 v1 Compat | 10-14 | 10-14 | Unchanged |
| 2.5 Frame Types | 8-11 | 10-14 | More types needed per doc 11 |
| **Total** | **75-95** | **77-98** | Slightly higher |

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial Phase 2 sprint plan |
| 1.1.0 | 2026-02-01 | Gap analysis, header size correction, frame type numbering alignment |
