# WRAITH Protocol v2 Migration Strategy

**Version:** 1.1.0
**Date:** 2026-02-01
**Status:** Planning Document
**Baseline:** v2.3.7 (2,148 tests, ~141,000 lines Rust, 12 client applications)

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Current State Analysis](#2-current-state-analysis)
3. [V2 Requirements Matrix](#3-v2-requirements-matrix)
4. [Phase Overview (Staged Release)](#4-phase-overview-staged-release)
    - 4.1 [Client Migration Priority](#41-client-migration-priority)
5. [Client Migration Matrix](#5-client-migration-matrix)
6. [Breaking Changes Inventory](#6-breaking-changes-inventory)
7. [Backwards Compatibility Strategy](#7-backwards-compatibility-strategy)
8. [Testing Strategy](#8-testing-strategy)
9. [Critical Path Analysis](#9-critical-path-analysis)
10. [Stage Gate Criteria](#10-stage-gate-criteria)
11. [Risk Register](#11-risk-register)

---

## 1. Executive Summary

### Scope

The v2 migration transforms WRAITH Protocol from a UDP-only, Linux-focused, classical-crypto protocol into a multi-transport, cross-platform, post-quantum-capable protocol with native group communication and real-time extensions. The migration touches all 8 protocol crates, the FFI layer, the CLI, and all 12 client applications.

### Staged Release Timeline

The migration follows a staged alpha/beta/release approach. The core insight is that the wire format (Phase 2) is the highest-risk breaking change -- validating it early de-risks everything downstream. By gating each stage, the project avoids committing to the full 976-1,242 SP scope until the riskiest changes are proven.

| Stage | Phases | Story Points | Duration | Gate |
|-------|--------|-------------|----------|------|
| **v3.0.0-alpha** | 1 (Crypto), 2 (Wire Format), 3 (Transport) | 257-329 SP | 7-8 weeks | Protocol tests pass, wire format finalized, perf within 10% of v1 |
| **v3.0.0-beta** | 7 (Obfuscation), 9 (Discovery/Session), 4 (Integration) | 268-352 SP | 5-7 weeks | v1-v2 interop validated, fuzz 1M+ iterations, integration suite green |
| **v3.0.0** | 8 (Group/RT, OPTIONAL), 5 (Clients), 6 (Release) | 451-561 SP | 8-11 weeks | All 12 clients migrated, security audit passed, docs complete |
| **Total** | | **976-1,242 SP** | **20-26 weeks** | |

### Risk Assessment Summary

- **HIGH**: Wire format breaking change affects all 12 clients and all packet processing
- **HIGH**: ML-KEM performance on mobile (Android ARM) may exceed battery budgets
- **MEDIUM**: Polymorphic format adds complexity to debugging/troubleshooting
- **MEDIUM**: 12 complete clients multiply migration effort vs. the 6 originally planned
- **LOW**: Existing ML-KEM-768 and HKDF-BLAKE3 code reduces crypto foundation risk

---

## 2. Current State Analysis

### Crate-by-Crate Breakdown

#### wraith-core (456 tests)

| Component | v1 State | v2 Requirement | Gap |
|-----------|----------|----------------|-----|
| `ConnectionId` | `u64` (8 bytes) at `session.rs:25` | `[u8; 16]` (128-bit) | BREAKING - ~60 usage sites |
| `FrameHeader` | 28 bytes, `u32` seq, `u16` payload_len | 24 bytes inner, `u64` seq, extensions | BREAKING - header format change |
| `FrameType` | 16 types (0x00-0x0F) | 50+ types across 7 categories | Extension needed |
| `FrameFlags` | 4 flags defined | 8 flags (add ECN, RTX, EXT, CMP) | Extension needed |
| `FRAME_HEADER_SIZE` | 28 bytes (`lib.rs:105`) | 24 bytes inner frame | Shrinks |
| Stream multiplexing | Complete (`stream.rs`) | Add QoS modes, priority, expedited | Enhancement |
| BBR congestion | Complete (`congestion.rs`) | Add BBRv2 8-phase probe | Enhancement |
| Session state machine | 4 states | 7 states (add REKEYING/MIGRATING/RESUMING/DRAINING) | Extension |
| Node API | Complete (`node/`) | Builder pattern, async-first | BREAKING |
| Connection migration | Complete (`migration.rs`) | Multi-transport migration | Enhancement |
| Group sessions | None | TreeKEM, topology modes | NEW |
| QoS/FEC | None | 4 QoS modes, 3 FEC algorithms | NEW |
| Session resumption | None | Ticket-based 0.5-RTT | NEW |
| Resource profiles | None | 5 profiles | NEW |

#### wraith-crypto (216 tests)

| Component | v1 State | v2 Requirement | Gap |
|-----------|----------|----------------|-----|
| X25519 | Complete (`x25519.rs`) | Unchanged (classical component) | None |
| ML-KEM-768 | Basic wrapper (`pq.rs`) | Hybrid combiner needed | PARTIAL |
| Elligator2 | Complete (`elligator.rs`) | Mandatory in v2 (was optional) | Config change |
| XChaCha20-Poly1305 | Complete (`aead/`) | Unchanged | None |
| BLAKE3 hash/KDF | Complete (`hash.rs`) | Labels need v2 update | Minor |
| HKDF | BLAKE3-based already | v2 key schedule (directional keys, wire format seed) | Enhancement |
| Double Ratchet | Complete (`ratchet.rs`) | Keep for application layer | None |
| Per-packet ratchet | None | BLAKE3 symmetric chain, per-packet advance | NEW |
| Ed25519 | Complete (`signatures.rs`) | Unchanged | None |
| ML-DSA-65 | None | Optional PQ signatures | NEW |
| Crypto suite negotiation | None | Suite A/B/C/D selection | NEW |
| TreeKEM | None | Group forward secrecy | NEW |
| Noise_XX handshake | Complete (`noise.rs`) | Extended with proof + PQ + extensions | Enhancement |

#### wraith-transport (183 tests)

| Component | v1 State | v2 Requirement | Gap |
|-----------|----------|----------------|-----|
| Transport trait | Partial (async) | Full trait with characteristics, MTU, close | Enhancement |
| UDP | Complete (sync + async) | Unchanged | None |
| AF_XDP | Complete with UMEM, rings, stats | Unchanged | None |
| io_uring | Complete | Unchanged | None |
| QUIC | Stub only | Full `quinn` integration | NEW |
| WebSocket | None (mimicry framing exists in obfuscation) | Full transport impl | NEW |
| TCP | None | Fallback transport | NEW |
| HTTP/2 | None | Firewall traversal | NEW |
| HTTP/3 | None | Modern deployment | NEW |
| TransportManager | None | Multi-transport with fallback | NEW |
| Transport migration | None (frame types exist) | Full state machine | NEW |
| Cross-platform | Linux-only for kernel bypass | Graceful fallback | Enhancement |

#### wraith-obfuscation (140 tests)

| Component | v1 State | v2 Requirement | Gap |
|-----------|----------|----------------|-----|
| Fixed padding (5 modes) | Complete | Replace with continuous distributions | BREAKING |
| Timing obfuscation (5 modes) | Complete | Add distribution matching modes | Enhancement |
| TLS mimicry | Complete | Integrate with probing resistance | Enhancement |
| WebSocket mimicry | Complete | Unchanged | None |
| DoH tunnel | Complete | Unchanged | None |
| Cover traffic | Complete | Enhanced decoy streams with mixing | Enhancement |
| Adaptive profiles | Complete | Integrate with resource profiles | Enhancement |
| Continuous padding | None | Uniform, HttpsEmpirical, Gaussian, Adaptive | NEW |
| Probing resistance | None | Proof-of-knowledge, service fronting | NEW |
| Entropy normalization | None | PredictableInsertion, Base64, JSON, HTTP | NEW |
| Decoy traffic mixing | None | Replace, Additive, Interleave strategies | NEW |

#### wraith-discovery (301 tests)

| Component | v1 State | v2 Requirement | Gap |
|-----------|----------|----------------|-----|
| Kademlia DHT | Complete | Privacy-enhanced (group-encrypted) | Enhancement |
| STUN | Complete | Unchanged | None |
| ICE (RFC 8445) | Complete | Unchanged | None |
| Relay | Complete | DERP-style with pub key routing | Enhancement |
| NAT traversal | Complete | Unchanged | None |

#### wraith-files (34 tests)

| Component | v1 State | v2 Requirement | Gap |
|-----------|----------|----------------|-----|
| io_uring I/O | Complete | Unchanged | None |
| Chunking | Complete | Content-defined chunking (optional) | Enhancement |
| Tree hashing | Complete | Unchanged (BLAKE3) | None |
| Reassembly | Complete | Unchanged | None |

#### wraith-ffi (111 tests)

| Component | v1 State | v2 Requirement | Gap |
|-----------|----------|----------------|-----|
| C-compatible API | Complete | Add hybrid crypto, multi-stream, builder pattern | Enhancement |
| JNI bindings | Complete | Update for v2 types | Enhancement |
| UniFFI bindings | Complete | Update for v2 types | Enhancement |

#### wraith-cli (87 tests)

| Component | v1 State | v2 Requirement | Gap |
|-----------|----------|----------------|-----|
| Full CLI | Complete | Add transport selection, crypto mode, compat flags | Enhancement |
| Config system | Complete | v2 config format | Enhancement |

---

## 3. V2 Requirements Matrix

| # | Requirement | Source | Status | Crate(s) | Client Impact | Phase | SP Est. |
|---|-------------|--------|--------|----------|---------------|-------|---------|
| R1 | Hybrid X25519 + ML-KEM-768 key exchange | Doc 01 s5.2, Doc 12 | PARTIAL (ML-KEM exists, no combiner) | wraith-crypto | All | 1 | 26-32 |
| R2 | HKDF-BLAKE3 with v2 labels | Doc 01 s5.4, Doc 12 | PARTIAL (BLAKE3 exists, labels differ) | wraith-crypto | None | 1 | 8-10 |
| R3 | Per-packet symmetric ratchet | Doc 01 s5.5.1 | NOT STARTED | wraith-crypto | None | 1 | 21-26 |
| R4 | ML-DSA-65 optional signatures | Doc 01 s5.1, Doc 12 | NOT STARTED | wraith-crypto | None | 1 | 8-12 |
| R5 | Crypto suite negotiation (A/B/C/D) | Doc 01 s5.1, Doc 12 | NOT STARTED | wraith-crypto | None | 1 | 13-16 |
| R6 | 128-bit ConnectionId | Doc 03 s2.1, Doc 11 | NOT STARTED | wraith-core | ALL 12 clients | 2 | 16-20 |
| R7 | Polymorphic wire format | Doc 01 s4.1, Doc 11 | NOT STARTED | wraith-core | None (internal) | 2 | 26-32 |
| R8 | Extended frame types (50+) | Doc 01 s4.3, Doc 11 | NOT STARTED | wraith-core | wraith-chat (group), wraith-stream (QoS) | 2 | 10-14 |
| R9 | Frame extension framework | Doc 01 s4.2 | NOT STARTED | wraith-core | None (internal) | 2 | 8-10 |
| R10 | v1 wire format compatibility | Doc 03 s10, Doc 11 | NOT STARTED | wraith-core | None | 2 | 10-14 |
| R11 | Pluggable transport abstraction | Doc 01 s7.1 | PARTIAL (trait exists) | wraith-transport | None (internal) | 3 | 16-20 |
| R12 | WebSocket transport | Doc 01 s7.2.2 | NOT STARTED | wraith-transport | wraith-chat | 3 | 13-16 |
| R13 | QUIC transport | Doc 01 s7 | STUB | wraith-transport | All | 3 | 16-20 |
| R14 | TransportManager with fallback | Doc 01 s7.3 | NOT STARTED | wraith-transport | All | 3 | 21-26 |
| R15 | Transport migration state machine | Doc 01 s7, Doc 03 s7.3 | NOT STARTED | wraith-transport | All | 3 | 18-23 |
| R16 | HTTP/2 + HTTP/3 transports | Doc 01 s7 | NOT STARTED | wraith-transport | None | 3 | 6-10 |
| R17 | Continuous padding distributions | Doc 01 s6.3 | NOT STARTED | wraith-obfuscation | None (internal) | 7 | 21-26 |
| R18 | Probing resistance | Doc 01 s6.5 | NOT STARTED | wraith-obfuscation | None (internal) | 7 | 26-32 |
| R19 | Entropy normalization | Doc 01 s6.6 | NOT STARTED | wraith-obfuscation | None (internal) | 7 | 18-23 |
| R20 | Enhanced decoy traffic | Doc 01 s6.7 | NOT STARTED | wraith-obfuscation | None (internal) | 7 | 18-23 |
| R21 | Timing distribution matching | Doc 01 s6.4 | PARTIAL (5 modes exist) | wraith-obfuscation | None (internal) | 7 | 13-16 |
| R22 | Group communication (TreeKEM) | Doc 01 s13 | NOT STARTED | wraith-core, wraith-crypto | wraith-chat, wraith-share, wraith-mesh | 8 | 52-64 |
| R23 | QoS modes (4 modes) | Doc 01 s14.1 | NOT STARTED | wraith-core | wraith-stream, wraith-chat | 8 | 21-26 |
| R24 | Forward Error Correction | Doc 01 s14.2 | NOT STARTED | wraith-core | wraith-stream | 8 | 18-23 |
| R25 | Session resumption (tickets) | Doc 01 s8.3 | NOT STARTED | wraith-core | All | 8 | 13-16 |
| R26 | Resource profiles (5 profiles) | Doc 01 Appendix C | NOT STARTED | wraith-core | All | 8 | 5-7 |
| R27 | Privacy-enhanced DHT | Doc 01 s12 | NOT STARTED | wraith-discovery | wraith-mesh | 9 | 21-26 |
| R28 | Enhanced session states (7 states) | Doc 01 s8.1 | PARTIAL (4 states exist) | wraith-core | None (internal) | 9 | 18-23 |
| R29 | DERP-style relay | Doc 01 s11.3 | NOT STARTED | wraith-discovery | wraith-mesh | 9 | 13-16 |
| R30 | FFI v2 update | Doc 16 | NOT STARTED | wraith-ffi | wraith-android, wraith-ios | 5 | 21-26 |
| R31 | CLI v2 update | Doc 16 | NOT STARTED | wraith-cli | None | 5 | 18-23 |
| R32 | All 12 client migrations | Doc 09 | NOT STARTED | clients/* | All 12 | 5 | 136-166 |
| R33 | v1 compatibility mode (90-day) | Doc 03 s10, Doc 15 | NOT STARTED | wraith-core | All | 4, 6 | 10-14 |
| R34 | Performance targets (500 Mbps UDP, 40 Gbps AF_XDP) | Doc 13 | PARTIAL (300 Mbps achieved) | All | None | 4 | 21-26 |
| R35 | Security validation (fuzzing, side-channel) | Doc 14, Doc 17 | NOT STARTED | All | None | 4 | 26-32 |

---

## 4. Phase Overview (Staged Release)

The migration is organized into three stages, each with explicit gate criteria that must be met before proceeding. This structure validates the riskiest changes (wire format, crypto) early before committing to the full scope.

### Stage 1: v3.0.0-alpha -- Core Protocol (257-329 SP, 7-8 weeks)

**Objective:** Establish the new cryptographic foundation, wire format, and multi-transport layer. The wire format (Phase 2) is the single highest-risk breaking change in the entire migration -- getting it right first de-risks everything downstream.

**Success Criteria (Alpha Gate):** All protocol-level tests pass, wire format finalized, performance within 10% of v1 baselines.

**Client Validation Scope:** wraith-cli, wraith-transfer, wraith-chat only (3 clients). These are sufficient to validate the core protocol end-to-end without the overhead of migrating all 12 clients.

**What Gets Validated:** Hybrid PQ key exchange, per-packet ratchet, polymorphic wire format round-trip, multi-transport fallback, v1 compatibility shim.

**Phases Included:**

#### Phase 1: Crypto Foundation (90-116 SP, 2-3 weeks)

**Objective:** Establish post-quantum hybrid cryptography, per-packet ratchet, and algorithm agility.

**Prerequisites:** None (foundation phase).

**Sprint Breakdown:**
- 1.1: ML-KEM-768 completion (8-12 SP) -- pq.rs already has basic wrapper
- 1.2: Hybrid KEM combiner (26-32 SP) -- domain-separated BLAKE3 combination
- 1.3: HKDF label migration (8-10 SP) -- already BLAKE3, just label/schedule changes
- 1.4: Per-packet symmetric ratchet (21-26 SP) -- new BLAKE3 chain separate from Double Ratchet
- 1.5: ML-DSA-65 optional signatures (8-12 SP) -- new ml-dsa crate
- 1.6: Integration (6-8 SP) -- CryptoContext v2 facade
- 1.7: Suite negotiation (13-16 SP) -- NEW: A/B/C/D suite framework

**Deliverables:** Hybrid KEM, per-packet ratchet, ML-DSA-65, suite negotiation, CryptoContext v2.

**Risk Factors:** ML-KEM crate maturity, timing side-channel in hybrid combination.

#### Phase 2: Wire Format (77-98 SP, 2 weeks)

**Objective:** Implement 128-bit CIDs, polymorphic encoding, extended frame types.

**Prerequisites:** Phase 1 (session secret for format derivation).

**Sprint Breakdown:**
- 2.1: 128-bit ConnectionId (16-20 SP)
- 2.2: Frame header reconciliation (15-18 SP) -- 28->24 byte inner frame
- 2.3: Polymorphic format (26-32 SP) -- field permutation, XOR mask, byte swap
- 2.4: v1 compatibility mode (10-14 SP)
- 2.5: Extended frame types and flags (10-14 SP)

**Deliverables:** New wire format, polymorphic encoding, v1 compat.

**Risk Factors:** Extreme sensitivity -- all packet processing changes.

#### Phase 3: Transport (90-115 SP, 2-3 weeks)

**Objective:** Multi-transport abstraction with WebSocket, QUIC, TCP, connection migration.

**Prerequisites:** Phase 2 (wire format for frames).

**Deliverables:** Unified Transport trait, TransportManager, WS/QUIC/TCP/HTTP transports, migration state machine.

**Go/No-Go for Beta:** Before proceeding to Stage 2, the alpha gate criteria (Section 10) must be satisfied. If the wire format proves problematic, this is the point to iterate -- not after 12 clients have been migrated.

---

### Stage 2: v3.0.0-beta -- Extended Protocol (268-352 SP, 5-7 weeks)

**Objective:** Complete the remaining protocol-layer work (obfuscation, discovery) and run comprehensive integration testing. Validate v1-to-v2 interoperability before committing to the client migration effort.

**Success Criteria (Beta Gate):** Integration tests pass, v1-to-v2 interop validated, security fuzzing complete (1M+ iterations), performance benchmarks meet targets.

**Client Validation Scope:** All 12 clients tested against beta in v1 compatibility mode to surface issues early, before full migration begins.

**What Gets Validated:** Continuous padding statistical properties, probing resistance under active probes, privacy-enhanced DHT, DERP relay functionality, end-to-end v1-v2 interop.

**Phases Included:**

#### Phase 7: Obfuscation Upgrades (110-140 SP, 2-3 weeks)

**Objective:** Continuous padding, probing resistance, entropy normalization, enhanced decoys.

**Prerequisites:** Phase 1 (crypto for probing proof), Phase 2 (polymorphic format).

#### Phase 9: Discovery & Session (55-70 SP, 1-2 weeks)

**Objective:** Privacy-enhanced DHT, expanded session states, DERP relay.

**Prerequisites:** Phase 1 (crypto primitives). Note: Phase 8 dependency removed -- Phase 9 no longer requires group encrypted announcements; privacy-enhanced DHT uses pairwise encryption available from Phase 1.

#### Phase 4: Integration Testing (103-142 SP, 2-3 weeks)

**Objective:** Comprehensive validation of all v2 features implemented in Stages 1 and 2.

**Prerequisites:** Phases 1-3, 7, 9.

**Go/No-Go for Release:** Before proceeding to Stage 3, the beta gate criteria (Section 10) must be satisfied. The beta gate is the last checkpoint before the expensive client migration work begins.

---

### Stage 3: v3.0.0 -- Full Release (451-561 SP, 8-11 weeks)

**Objective:** Migrate all 12 clients, complete the release process, and optionally implement group/real-time extensions.

**Success Criteria (Release Gate):** All 12 clients migrated, security audit passed, documentation complete, performance certification met.

**Client Validation Scope:** Full migration of all 12 clients, prioritized per Section 4.1.

**Phases Included:**

#### Phase 8: Group & Real-Time (130-160 SP, 3-4 weeks) -- OPTIONAL for v3.0.0

**Objective:** TreeKEM group sessions, QoS modes, FEC, session resumption.

**Prerequisites:** Phase 1 (crypto for TreeKEM), Phase 2 (frame types).

**DEFERRAL NOTE:** This phase is OPTIONAL for v3.0.0 and can be deferred to v3.1.0 without blocking the core v2 migration. TreeKEM (52-64 SP) is the single highest-risk item in this phase, and group communication may not be immediately needed by all deployments. Session resumption (13-16 SP) and resource profiles (5-7 SP) are lower-risk and could be included in v3.0.0 independently if desired. If Phase 8 is deferred entirely, the release stage reduces to 321-401 SP and 5-8 weeks.

#### Phase 5: Client Updates (236-301 SP, 4-5 weeks)

**Objective:** Migrate all 12 clients and FFI/CLI. Client migration should follow the priority order defined in Section 4.1.

**Prerequisites:** Phase 4 (validated protocol).

#### Phase 6: Release (85-100 SP, 1-2 weeks)

**Objective:** Documentation, security audit, performance certification, v3.0.0 release.

**Prerequisites:** Phase 5 (all clients migrated).

---

### 4.1 Client Migration Priority

Clients are ranked by migration priority. P0 clients are validated during alpha to prove the protocol works end-to-end. P1 clients (security tooling) must be on the current protocol version to maintain operational integrity. P3/P4 clients can tolerate running in v1 compatibility mode longer.

| Priority | Clients | Count | Rationale |
|----------|---------|-------|-----------|
| **P0** (alpha validation) | wraith-cli, wraith-transfer, wraith-chat | 3 | Minimum viable set for protocol validation |
| **P1** (critical) | wraith-recon, wraith-redops | 2 | Security tooling must be on current protocol version |
| **P2** (high) | wraith-android, wraith-ios | 2 | Mobile users; FFI/binding updates required |
| **P3** (medium) | wraith-sync, wraith-share, wraith-stream, wraith-vault, wraith-publish | 5 | Can operate in v1 compat mode during transition |
| **P4** (low) | wraith-mesh | 1 | Can defer to v3.1.0 if needed |

During Phase 5, clients should be migrated in priority order. P0 clients are already validated during alpha; Phase 5 work for P0 clients is limited to final polish and ensuring they work against the beta-validated protocol. The bulk of Phase 5 effort is P1-P3 migrations.

---

## 5. Client Migration Matrix

| Client | wraith-core | wraith-crypto | wraith-transport | wraith-obfuscation | wraith-discovery | wraith-files | Impact Level |
|--------|:-----------:|:-------------:|:----------------:|:------------------:|:----------------:|:------------:|:------------:|
| wraith-transfer | X | | | | | | LOW |
| wraith-chat | X | X | X | | X | X | HIGH |
| wraith-android | X | X | X | | X | X | HIGH |
| wraith-ios | X | X | X | | X | X | HIGH |
| wraith-sync | X | X | | | | X | MEDIUM |
| wraith-share | X | X | | | X | X | MEDIUM |
| wraith-stream | X | X | | | X | X | MEDIUM |
| wraith-mesh | X | | | | X | | MEDIUM |
| wraith-publish | X | X | | | X | | MEDIUM |
| wraith-vault | X | X | | | X | X | MEDIUM |
| wraith-recon | X | X | X | X | X | X | CRITICAL |
| wraith-redops | X | X | X | | | | CRITICAL |

**Key breaking changes affecting clients:**
- `ConnectionId(u64)` -> `ConnectionId([u8; 16])` -- ALL clients
- `FrameHeader` struct changes -- clients that construct frames directly
- `Session::new()` -> `Session::builder()` -- ALL clients
- Async-only API -- clients using sync wrappers

---

## 6. Breaking Changes Inventory

### Type Changes

| Type | v1 | v2 | Locations Affected |
|------|----|----|-------------------|
| `ConnectionId` | `ConnectionId(u64)` | `ConnectionId { bytes: [u8; 16] }` | ~60 sites in wraith-core + all clients |
| `FrameHeader.sequence` | `u32` | `u64` | Frame parsing, SIMD code, builder |
| `FrameHeader.payload_len` | `u16` | Replaced by offset/extension framework | Frame parsing |
| `FRAME_HEADER_SIZE` | 28 bytes | 24 bytes (inner) | ~50 usage sites |
| `FrameType` enum | 16 variants (0x00-0x0F) | 50+ variants (0x00-0x6F) | Frame dispatch, lookup table |
| `SessionState` | 4 states | 7 states | Session state machine |

### API Changes

| API | v1 Signature | v2 Signature | Impact |
|-----|-------------|--------------|--------|
| Session creation | `Session::new(cid, addr, secret)` | `Session::builder()...build()` | All clients |
| send/recv | Mixed sync/async | Async-only (`send().await`) | All clients |
| Padding | `PaddingMode` enum (5 fixed classes) | `PaddingDistribution` (continuous) | wraith-obfuscation consumers |
| KDF labels | `"wraith-v1-*"` | `"wraith-v2-*"` | Wire incompatible |
| Key derivation | Symmetric keys | Directional keys (i2r/r2i) | All session crypto |

### Wire Format Changes

| Aspect | v1 | v2 | Compatibility |
|--------|----|----|--------------|
| Outer packet CID | 8 bytes | Polymorphic (varies) | INCOMPATIBLE |
| Inner frame header | 28 bytes, fixed layout | 24 bytes, fixed layout | INCOMPATIBLE |
| Handshake | `e, ee, s, es, se` | `proof, e, [pq_pk], ee, [pq_ct], s, es, se, extensions` | INCOMPATIBLE |
| Padding | Fixed size classes (64/256/512/1024/1472/8960) | Continuous distribution | INCOMPATIBLE |
| Rekey frame | X25519 ephemeral only | X25519 + optional ML-KEM + ratchet seq | INCOMPATIBLE |

---

## 7. Backwards Compatibility Strategy

### Version Negotiation

Per doc 03 section 1.3, v2 introduces a version byte in the proof packet:

```
v2 Proof Packet:
[Version (1B)] [Timestamp (8B)] [Random (16B)] [Proof (32B)] ...
```

v1 packets have no version field; detection is heuristic based on packet structure.

### Dual-Stack Period

Per doc 03 section 10.5, three approaches are supported:

1. **Gateway approach** (recommended for gradual rollout): Deploy v1<->v2 gateway that translates between versions. Estimated ~2,000 lines for a standalone gateway binary.

2. **Dual-stack** in code: `WireFormat` enum with V1/V2/V2Polymorphic variants, format auto-detection, negotiation during handshake. This is Phase 2 Sprint 2.4 (10-14 SP).

3. **Feature flags**: v2 server with `v1-compat` feature flag that reduces to classical-only, fixed wire format, v1-compatible handshake.

### Deprecation Timeline

Per Phase 6 Sprint 6.6:
- v3.0.0 release: v1 compat mode available, deprecated with warnings
- +30 days: First deprecation reminder
- +60 days: Final reminder
- +90 days (v4.0.0): v1 compat removed

---

## 8. Testing Strategy

### Test Pyramid

| Level | v1 Count | v2 Target | New Tests |
|-------|----------|-----------|-----------|
| Unit | ~1,800 | ~2,800 | ~1,000 |
| Integration | ~200 | ~400 | ~200 |
| E2E | ~50 | ~100 | ~50 |
| Property-based | ~50 | ~200 | ~150 |
| Fuzz targets | 0 | 7+ | 7+ |
| Security | ~50 | ~100 | ~50 |
| **Total** | **~2,148** | **~3,600** | **~1,450** |

### Phase-Specific Testing

| Phase | Test Focus | Method |
|-------|-----------|--------|
| 1 (Crypto) | KAT vectors, property tests, timing analysis | NIST test vectors, `proptest`, `dudect` |
| 2 (Wire Format) | Round-trip encoding, polymorphic determinism, fuzzing | `proptest`, `cargo-fuzz`, SIMD correctness |
| 3 (Transport) | Multi-transport fallback, migration, cross-platform | Integration tests, CI matrix (Linux/macOS/Windows) |
| 4 (Integration) | v1<->v2 interop, performance benchmarks, security | Docker multi-node, `criterion`, fuzzing |
| 7 (Obfuscation) | Statistical distribution tests, probing resistance | Chi-squared, KS tests, active probe simulation |
| 8 (Group) | TreeKEM correctness, QoS mode validation | Property tests, network simulation |

### CI/CD Pipeline

```
Unit Tests (all platforms) --> Integration Tests --> Security Tests
                                                        |
                                    Performance Benchmarks (regression gate)
                                                        |
                                    Interop Tests (v1 <-> v2)
                                                        |
                                    Fuzz Testing (1M+ iterations)
```

---

## 9. Critical Path Analysis

### Staged Dependency Graph

```
                         STAGE 1: v3.0.0-alpha
                  ┌──────────────────────────────────┐
                  │                                    │
Phase 1 (Crypto) ──> Phase 2 (Wire Format) ──> Phase 3 (Transport)
                  │                                    │
                  └──────────────────────────────────┘
                                  │
                          [ALPHA GATE]  <-- Wire format validated here
                                  │
                         STAGE 2: v3.0.0-beta
                  ┌──────────────────────────────────┐
                  │                                    │
                  ├──> Phase 7 (Obfuscation) ────────┤
                  │                                    │
                  ├──> Phase 9 (Discovery/Session) ──┤
                  │                                    │
                  └──> Phase 4 (Integration) ────────┘
                                  │
                          [BETA GATE]  <-- v1-v2 interop validated here
                                  │
                         STAGE 3: v3.0.0
                  ┌──────────────────────────────────┐
                  │                                    │
                  ├──> Phase 8 (Group/RT) [OPTIONAL] ┤
                  │                                    │
                  ├──> Phase 5 (Clients, P0->P4) ────┤
                  │                                    │
                  └──> Phase 6 (Release) ────────────┘
```

The key insight of the staged approach is that the alpha gate validates the riskiest path (wire format breaking change) before the project commits to the full scope. If the wire format proves problematic at the alpha gate, iteration cost is limited to 257-329 SP rather than the full 976-1,242 SP.

### Critical Path

The longest dependency chain through stages:

**Stage 1:** Phase 1 (3w) -> Phase 2 (2w) -> Phase 3 (3w) = 8 weeks
**Stage 2:** Phase 7 (3w) || Phase 9 (2w) -> Phase 4 (3w) = 5-6 weeks
**Stage 3:** Phase 5 (5w) -> Phase 6 (2w) = 7 weeks

**Total critical path: ~20 weeks** (sequential stages with gate reviews between each)

### Parallelization Opportunities

**Within Stage 2:** Phases 7 and 9 can run in parallel. Phase 4 must wait for both to complete. With 2 developers, Stage 2 takes 5 weeks (Phase 7 || Phase 9, then Phase 4).

**Within Stage 3:** Phase 8 (if included) can run in parallel with the early Phase 5 client migrations (P0/P1). Phase 5 clients can be parallelized across multiple developers since each client migration is largely independent.

**With 2 developers:** ~20 weeks (stage gates add ~1 week each)
**With 3 developers:** ~17 weeks (Phase 7/9 parallel, client migrations parallel)
**With 4 developers:** ~15 weeks (max parallelization within each stage)

### Bottlenecks

1. **Phase 2 (Wire Format)**: The highest-risk single phase. Every subsequent phase depends on the new wire format. The staged approach specifically gates on this: alpha validation proves the format before downstream work begins.
2. **Alpha Gate**: A failed alpha gate (wire format issues, performance regression) blocks all subsequent work. Budget 1 week for gate review and potential rework.
3. **Phase 5 (Client Updates)**: 12 clients at ~236-301 SP is the largest single phase. Can be parallelized across client developers following the priority order in Section 4.1.
4. **Phase 4 (Integration Testing)**: Gate between protocol implementation and client migration. Must wait for all Stage 2 implementation phases.

---

## 10. Stage Gate Criteria

Each stage gate is a formal go/no-go decision point. All criteria must be met before proceeding to the next stage. Failed gates require remediation before advancement.

### Alpha Gate (after Stage 1, before Stage 2)

| # | Criterion | Verification Method |
|---|-----------|-------------------|
| AG1 | Crypto KAT tests pass for all 4 suites (A/B/C/D) | NIST test vectors, `cargo test -p wraith-crypto` |
| AG2 | Wire format round-trip: encode -> decode for all 50+ frame types | Property tests with `proptest`, 100K+ cases |
| AG3 | Polymorphic format determinism: same session secret produces same permutation | Unit tests with fixed seeds |
| AG4 | Transport fallback: QUIC -> WebSocket -> TCP -> UDP automatic degradation | Integration test with simulated transport failures |
| AG5 | Performance within 10% of v1 baselines for frame build, AEAD, handshake | `criterion` benchmarks compared against v1 baseline |
| AG6 | 3 P0 clients (wraith-cli, wraith-transfer, wraith-chat) functional on v2 protocol | End-to-end file transfer and message exchange tests |
| AG7 | v1 compatibility mode: v2 node can communicate with v1 node | Cross-version integration test |

### Beta Gate (after Stage 2, before Stage 3)

| # | Criterion | Verification Method |
|---|-----------|-------------------|
| BG1 | v1-to-v2 interop: bidirectional communication across version boundary | Docker multi-node test (v1 and v2 nodes) |
| BG2 | Fuzz testing: 1M+ iterations on wire format parser, no crashes | `cargo-fuzz` with ASAN/MSAN |
| BG3 | Obfuscation statistical tests: padding distributions pass chi-squared and KS tests | Statistical test suite with p > 0.05 threshold |
| BG4 | Probing resistance: no information leakage under active probe simulation | Active probe test suite (10 probe strategies) |
| BG5 | Integration test suite green: all Phase 4 tests pass | `cargo test --workspace` + integration suite |
| BG6 | Performance benchmarks meet targets: 500 Mbps UDP, frame build within 10% of v1 | `criterion` benchmarks, CI performance gate |
| BG7 | All 12 clients tested against beta in v1 compat mode (no regressions) | Existing test suites run against v2 node in compat mode |

### Release Gate (after Stage 3, before v3.0.0 tag)

| # | Criterion | Verification Method |
|---|-----------|-------------------|
| RG1 | All 12 clients migrated to v2 native protocol | Per-client test suites pass on v2 |
| RG2 | Security audit passed (external or thorough internal review) | Audit report with no HIGH/CRITICAL findings open |
| RG3 | Performance certification: all benchmarks meet or exceed v1 baselines | Full benchmark suite, published results |
| RG4 | Documentation complete: API docs, migration guide, changelog | `cargo doc` clean, migration guide reviewed |
| RG5 | v1 compatibility mode functional with 90-day deprecation timeline configured | Deprecation warning tests, timeline configuration |
| RG6 | Zero clippy warnings, zero test failures, zero known security vulnerabilities | `cargo clippy -- -D warnings`, `cargo test --workspace` |

---

## 11. Risk Register

| # | Risk | Probability | Impact | Severity | Mitigation | Phase |
|---|------|-------------|--------|----------|------------|-------|
| R1 | ML-KEM crate bugs or API changes | Medium | High | HIGH | Pin version, extensive KAT testing, fallback to classical-only | 1 |
| R2 | ML-KEM performance on mobile ARM | Medium | Medium | MEDIUM | Early benchmarking on device, classical-only fallback for constrained | 1, 5 |
| R3 | Polymorphic format timing side-channel | Low | High | MEDIUM | Constant-time encode/decode, timing analysis in Phase 4 | 2 |
| R4 | Wire format change breaks all clients | Certain | High | HIGH | Phased rollout, v1 compat mode, compat shims, comprehensive testing | 2, 5 |
| R5 | Transport migration packet loss | Medium | Medium | MEDIUM | Queue packets during migration, timeout recovery, integration tests | 3 |
| R6 | QUIC (quinn) integration complexity | Medium | Low | LOW | Use well-tested crate, start with basic transport, enhance iteratively | 3 |
| R7 | Performance regression from added crypto overhead | Medium | Medium | MEDIUM | Continuous benchmarking, performance CI gate, profiling | 4 |
| R8 | 12 client migrations (was planned for 6) | Certain | Medium | MEDIUM | Prioritize critical clients first, compat shims reduce urgency | 5 |
| R9 | TreeKEM implementation correctness | Medium | High | HIGH | Extensive property tests, compare with MLS reference implementation | 8 |
| R10 | Probing resistance false positives (blocking legitimate clients) | Medium | High | HIGH | Configurable clock skew tolerance, gradual rollout, monitoring | 7 |
| R11 | Cross-platform inconsistencies | Medium | Medium | MEDIUM | CI matrix, platform-specific tests, conditional compilation | 3, 5 |
| R12 | Security audit findings blocking release | Medium | High | HIGH | Schedule audit early in Phase 6, buffer time for fixes | 6 |
| R13 | WASM target compilation issues | Medium | Low | LOW | Defer WASM to post-v3.0.0 if blocking, `no_std` core already exists | 6 |
| R14 | spectre-implant (no_std, excluded workspace) breakage | Low | Medium | LOW | Separate test/build pipeline, minimal protocol surface | 5 |

---

## Appendix A: Story Point Summary

### By Stage

| Stage | Min SP | Max SP | Duration |
|-------|--------|--------|----------|
| **v3.0.0-alpha** (Phases 1, 2, 3) | **257** | **329** | **7-8 weeks** |
| **v3.0.0-beta** (Phases 7, 9, 4) | **268** | **352** | **5-7 weeks** |
| **v3.0.0** (Phases 8, 5, 6) | **451** | **561** | **8-11 weeks** |
| **v3.0.0 without Phase 8** (Phases 5, 6) | **321** | **401** | **5-8 weeks** |
| **Total** | **976** | **1,242** | **20-26 weeks** |
| **Total without Phase 8** | **846** | **1,082** | **17-23 weeks** |

### By Phase

| Phase | Stage | Min SP | Max SP | Duration |
|-------|-------|--------|--------|----------|
| 1 - Crypto Foundation | alpha | 90 | 116 | 2-3 weeks |
| 2 - Wire Format | alpha | 77 | 98 | 2 weeks |
| 3 - Transport | alpha | 90 | 115 | 2-3 weeks |
| 7 - Obfuscation | beta | 110 | 140 | 2-3 weeks |
| 9 - Discovery & Session | beta | 55 | 70 | 1-2 weeks |
| 4 - Integration Testing | beta | 103 | 142 | 2-3 weeks |
| 8 - Group & Real-Time (OPTIONAL) | release | 130 | 160 | 3-4 weeks |
| 5 - Client Updates | release | 236 | 301 | 4-5 weeks |
| 6 - Release | release | 85 | 100 | 1-2 weeks |
| **Total** | | **976** | **1,242** | **20-26 weeks** |

## Appendix B: Estimated New Code Volume

| Phase | New Lines (Rust) | Modified Lines | Test Lines |
|-------|-----------------|----------------|------------|
| 1 | ~2,500 | ~500 | ~2,000 |
| 2 | ~2,000 | ~1,500 | ~1,500 |
| 3 | ~3,500 | ~500 | ~2,000 |
| 7 | ~3,000 | ~500 | ~2,000 |
| 8 | ~4,000 | ~300 | ~3,000 |
| 9 | ~1,500 | ~500 | ~1,500 |
| 4 | ~500 | ~200 | ~5,000 |
| 5 | ~3,000 | ~8,000 | ~2,000 |
| 6 | ~500 | ~2,000 | ~500 |
| **Total** | **~20,500** | **~14,000** | **~19,500** |

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.1.0 | 2026-02-01 | Restructured around staged alpha/beta/release approach; added stage gate criteria (Section 10); added client migration priority (Section 4.1); marked Phase 8 as optional for v3.0.0; updated critical path for staged model; reorganized Appendix A by stage |
| 1.0.0 | 2026-02-01 | Initial strategy document |
