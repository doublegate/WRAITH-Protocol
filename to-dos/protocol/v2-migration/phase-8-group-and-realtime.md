# Phase 8: Group Communication & Real-Time Extensions

**Parent:** [v2 Migration Master Plan](../v2-migration-master.md)
**Version:** 1.0.0
**Status:** Planning (Post-v2.3.7)
**Story Points:** 130-160 SP
**Duration:** 3-4 weeks
**Dependencies:** Phase 2 (Wire Format for new frame types), Phase 1 (Crypto for TreeKEM)

---

## Executive Summary

Phase 8 implements the two major new feature categories in v2: native group communication with TreeKEM key management, and real-time extensions including QoS modes, FEC, and datagram support. These features are specified in doc 01 sections 13-14.

### Objectives

1. Implement group session management with TreeKEM (per doc 01, section 13)
2. Implement QoS modes: Reliable, UnreliableOrdered, UnreliableUnordered, PartiallyReliable
3. Implement Forward Error Correction (XOR, Reed-Solomon)
4. Add datagram frame support for unreliable messaging
5. Add session resumption with tickets (per doc 01, section 8.3)
6. Add resource profiles (Performance, Balanced, Constrained, Stealth, Metered)

---

## Current Implementation State

| Component | Status | Notes |
|-----------|--------|-------|
| Stream multiplexing | COMPLETE | `wraith-core/src/stream.rs` |
| BBR congestion | COMPLETE | `wraith-core/src/congestion.rs` |
| Flow control | COMPLETE | Window-based in session/stream |
| Connection migration | COMPLETE | `wraith-core/src/migration.rs` |
| Group communication | NOT STARTED | wraith-chat has Sender Keys but not protocol-level group |
| QoS modes | NOT STARTED | Only Reliable mode exists |
| FEC | NOT STARTED | - |
| Datagram support | NOT STARTED | - |
| Session resumption | NOT STARTED | - |
| Resource profiles | NOT STARTED | - |

---

## Sprint Breakdown

### Sprint 8.1: Group Session Framework (26-32 SP)

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 8.1.1 | Define `GroupSession` struct | 3 | High |
| 8.1.2 | Define `GroupTopology` enum (Centralized, FullMesh, Tree, Gossip) | 3 | High |
| 8.1.3 | Implement GROUP_JOIN/LEAVE/REKEY frame handling | 5 | High |
| 8.1.4 | Implement member management (add/remove/roles) | 5 | High |
| 8.1.5 | Implement pairwise session tracking | 3 | High |
| 8.1.6 | Group announcement via DHT (wraith-discovery integration) | 5 | High |
| 8.1.7 | Unit tests | 5 | High |

**Code Location:** `crates/wraith-core/src/group/`

### Sprint 8.2: TreeKEM Group Key Management (26-32 SP)

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 8.2.1 | Define `TreeKem` struct with binary tree | 5 | High |
| 8.2.2 | Implement leaf secret generation | 3 | High |
| 8.2.3 | Implement path secret derivation | 5 | High |
| 8.2.4 | Implement `self_update()` with path encryption | 5 | High |
| 8.2.5 | Implement `process_update()` for receiving updates | 5 | High |
| 8.2.6 | Implement `group_key()` derivation from root | 2 | High |
| 8.2.7 | Member add/remove with tree restructuring | 5 | High |
| 8.2.8 | Unit tests (correctness, forward secrecy verification) | 5 | High |

**Code Location:** `crates/wraith-crypto/src/treekem/`

### Sprint 8.3: QoS Modes (21-26 SP)

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 8.3.1 | Define `QosMode` enum and `QosConfig` | 2 | High |
| 8.3.2 | Implement UnreliableOrdered mode (drop old) | 5 | High |
| 8.3.3 | Implement UnreliableUnordered mode | 3 | Medium |
| 8.3.4 | Implement PartiallyReliable mode (limited retransmits) | 5 | Medium |
| 8.3.5 | QOS_UPDATE frame handling | 3 | High |
| 8.3.6 | Per-stream QoS configuration | 3 | High |
| 8.3.7 | Jitter buffer for ordered modes | 3 | Medium |
| 8.3.8 | Unit tests | 3 | High |

**Code Location:** `crates/wraith-core/src/qos/`

### Sprint 8.4: Forward Error Correction (18-23 SP)

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 8.4.1 | Define `FecConfig` and `FecAlgorithm` enum | 2 | Medium |
| 8.4.2 | Implement XOR FEC encoder/decoder | 3 | Medium |
| 8.4.3 | Implement Reed-Solomon encoder/decoder | 8 | Medium |
| 8.4.4 | FEC_REPAIR frame handling | 3 | Medium |
| 8.4.5 | FEC negotiation during stream open | 3 | Medium |
| 8.4.6 | Unit tests | 3 | Medium |

**Code Location:** `crates/wraith-core/src/fec/`

### Sprint 8.5: Session Resumption & Resource Profiles (18-23 SP)

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 8.5.1 | Define `ResumptionTicket` with encrypted params | 3 | High |
| 8.5.2 | Implement ticket creation and validation | 5 | High |
| 8.5.3 | Implement abbreviated handshake (0.5 RTT) | 5 | High |
| 8.5.4 | Define `ResourceProfile` enum (Performance/Balanced/Constrained/Stealth/Metered) | 2 | High |
| 8.5.5 | Implement profile-based configuration | 3 | High |
| 8.5.6 | DATAGRAM frame type support | 3 | Medium |
| 8.5.7 | Unit tests | 3 | High |

**Code Location:** `crates/wraith-core/src/session/resumption.rs`, `crates/wraith-core/src/profile.rs`

---

## Client Impact

- wraith-chat: Direct beneficiary of group communication (replace app-level Sender Keys with protocol-level)
- wraith-stream: Direct beneficiary of QoS modes (UnreliableOrdered for video, PartiallyReliable for audio)
- wraith-share: Benefits from group/swarm transfer improvements
- wraith-mesh: Benefits from group topology visualization
- All clients: Resource profiles for constrained/mobile environments

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-02-01 | Initial Phase 8 sprint plan |
