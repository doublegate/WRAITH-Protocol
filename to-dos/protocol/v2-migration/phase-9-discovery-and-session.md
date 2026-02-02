# Phase 9: Discovery Protocol & Session Management Upgrades

**Parent:** [v2 Migration Master Plan](../v2-migration-master.md)
**Version:** 1.0.0
**Status:** Planning (Post-v2.3.7)
**Story Points:** 55-70 SP
**Duration:** 1-2 weeks
**Dependencies:** Phase 1 (Crypto), Phase 8 (Group for group-encrypted announcements)

---

## Executive Summary

Phase 9 upgrades the discovery and session management layers for v2. This includes privacy-enhanced DHT with group-encrypted announcements, session state machine expansion (REKEYING, MIGRATING, RESUMING, DRAINING states), enhanced NAT traversal, and DERP-style relay protocol.

---

## Current Implementation State

| Component | Status | Notes |
|-----------|--------|-------|
| Kademlia DHT | COMPLETE | `wraith-discovery/src/` - full DHT with routing |
| STUN | COMPLETE | Server reflexive discovery |
| ICE (RFC 8445) | COMPLETE | Full ICE agent with candidate gathering |
| Relay | COMPLETE | Basic relay protocol |
| NAT traversal | COMPLETE | Hole punching with birthday attack |
| Session state machine | PARTIAL | CLOSED/CONNECTING/ESTABLISHED/CLOSED only |
| Privacy-enhanced DHT | NOT STARTED | Group-encrypted announcements per doc 01, section 12 |
| DERP-style relay | NOT STARTED | Relay with pub key routing per doc 01, section 11.3 |
| Enhanced session states | NOT STARTED | REKEYING, MIGRATING, RESUMING, DRAINING |

---

## Sprint Breakdown

### Sprint 9.1: Privacy-Enhanced DHT (21-26 SP)

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 9.1.1 | Implement group-encrypted DHT announcements | 5 | High |
| 9.1.2 | Implement group key-derived DHT lookup keys | 3 | High |
| 9.1.3 | Sign-then-encrypt announcement protocol | 5 | High |
| 9.1.4 | Integration with wraith-discovery Kademlia | 5 | High |
| 9.1.5 | Unit tests | 5 | High |

### Sprint 9.2: Session State Machine Expansion (18-23 SP)

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 9.2.1 | Add REKEYING state with transition logic | 3 | Critical |
| 9.2.2 | Add MIGRATING state with transition logic | 3 | Critical |
| 9.2.3 | Add RESUMING state with ticket handling | 5 | High |
| 9.2.4 | Add DRAINING state for graceful shutdown | 3 | High |
| 9.2.5 | State timeout handling | 2 | High |
| 9.2.6 | Unit tests (all state transitions) | 5 | Critical |

### Sprint 9.3: DERP-Style Relay (13-16 SP)

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 9.3.1 | Define relay frame types (SendPacket, RecvPacket, Subscribe, KeepAlive, PeerPresent, PeerGone) | 3 | Medium |
| 9.3.2 | Implement relay connection with public key routing | 5 | Medium |
| 9.3.3 | Relay encryption layer | 3 | Medium |
| 9.3.4 | Unit tests | 3 | Medium |

---

## Client Impact

- wraith-discovery: Primary target for DHT upgrades
- wraith-mesh: Benefits from relay protocol for topology discovery
- All clients: Session state machine improvements affect all protocol usage

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-02-01 | Initial Phase 9 sprint plan |
