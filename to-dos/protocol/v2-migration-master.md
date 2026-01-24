# WRAITH Protocol v2 Migration Master Plan

**Version:** 1.0.0
**Status:** Planning (Post-v2.3.0)
**Target Release:** v3.0.0
**Estimated Timeline:** 12-16 weeks
**Total Story Points:** 580-720 SP

---

## Executive Summary

This document serves as the master planning guide for migrating WRAITH Protocol from v1.x to v2, culminating in the v3.0.0 release. The migration introduces post-quantum cryptography, polymorphic wire formats, per-packet forward secrecy, and multi-transport abstraction while maintaining backward compatibility during the transition period.

### Key Objectives

1. **Quantum Resistance** - Hybrid X25519 + ML-KEM-768 key exchange
2. **Enhanced Forward Secrecy** - Per-packet ratcheting (vs per-minute in v1)
3. **Traffic Analysis Resistance** - Polymorphic wire format with session-derived field positions
4. **Multi-Transport Support** - UDP, TCP, WebSocket, QUIC, HTTP/2, HTTP/3
5. **Extended Wire Format** - 128-bit CIDs, 64-bit sequences, 24-byte headers
6. **Client Migration** - Update all 9+ client applications

---

## Phase Overview

| Phase | Focus | Story Points | Duration | Dependencies |
|-------|-------|--------------|----------|--------------|
| Phase 1 | Crypto Foundation | 95-120 SP | 2-3 weeks | None |
| Phase 2 | Wire Format | 75-95 SP | 2 weeks | Phase 1 |
| Phase 3 | Transport | 90-115 SP | 2-3 weeks | Phase 2 |
| Phase 4 | Integration Testing | 85-110 SP | 2-3 weeks | Phase 3 |
| Phase 5 | Client Updates | 150-180 SP | 3-4 weeks | Phase 4 |
| Phase 6 | Release | 85-100 SP | 1-2 weeks | Phase 5 |
| **Total** | | **580-720 SP** | **12-16 weeks** | |

---

## Phase Summaries

### Phase 1: Crypto Foundation (95-120 SP)

**Goal:** Implement hybrid post-quantum cryptography and per-packet forward secrecy.

**Key Deliverables:**
- [ ] ML-KEM-768 wrapper around `ml-kem` crate
- [ ] Hybrid key exchange combining X25519 + ML-KEM-768
- [ ] HKDF-BLAKE3 key derivation replacing HKDF-SHA256
- [ ] Per-packet ratchet replacing per-minute ratchet
- [ ] Optional ML-DSA-65 signature support
- [ ] Comprehensive test suite (95% crypto coverage)

**Critical Path Items:**
- Hybrid key combination security proof
- Constant-time implementation verification
- Memory zeroization on all key material

**Reference:** [Phase 1 Sprint Plan](v2-migration/phase-1-crypto-foundation.md)

---

### Phase 2: Wire Format (75-95 SP)

**Goal:** Implement expanded wire format with polymorphic encoding.

**Key Deliverables:**
- [ ] 128-bit ConnectionId (from 64-bit)
- [ ] 64-bit sequence numbers (from 32-bit)
- [ ] 24-byte frame header (from 20-byte)
- [ ] Polymorphic field encoding with session-derived positions
- [ ] XOR masking of header fields
- [ ] v1 compatibility encoding/decoding

**Critical Path Items:**
- Wire format backward compatibility
- Polymorphic format cryptographic binding to session
- SIMD optimization for encoding/decoding

**Reference:** [Phase 2 Sprint Plan](v2-migration/phase-2-wire-format.md)

---

### Phase 3: Transport (90-115 SP)

**Goal:** Implement multi-transport abstraction with connection migration.

**Key Deliverables:**
- [ ] Unified async `Transport` trait
- [ ] `TransportManager` for multi-transport handling
- [ ] Connection migration between transports
- [ ] WebSocket transport implementation
- [ ] QUIC transport implementation
- [ ] HTTP/2 and HTTP/3 transport support
- [ ] Cross-platform abstraction (Linux/macOS/Windows)

**Critical Path Items:**
- Transport negotiation protocol
- Migration handoff without packet loss
- Platform-specific transport availability

**Reference:** [Phase 3 Sprint Plan](v2-migration/phase-3-transport.md)

---

### Phase 4: Integration Testing (85-110 SP)

**Goal:** Comprehensive integration testing, benchmarking, and security validation.

**Key Deliverables:**
- [ ] v1 <-> v2 interoperability testing
- [ ] Version negotiation validation
- [ ] Performance benchmarking against v1 baseline
- [ ] Security validation (fuzzing, property-based testing)
- [ ] Compatibility mode testing (90-day window)
- [ ] Migration path validation

**Critical Path Items:**
- No regression in v1 functionality
- Performance targets met (see below)
- All security assertions verified

**Reference:** [Phase 4 Sprint Plan](v2-migration/phase-4-integration.md)

---

### Phase 5: Client Updates (150-180 SP)

**Goal:** Migrate all client applications to v2 protocol.

**Key Deliverables:**
- [ ] wraith-cli v2 migration
- [ ] wraith-transfer (Tauri) v2 migration
- [ ] wraith-chat (Tauri) v2 migration
- [ ] wraith-sync (Tauri) v2 migration
- [ ] wraith-android (Kotlin/JNI) v2 migration
- [ ] wraith-ios (Swift/UniFFI) v2 migration
- [ ] wraith-ffi API updates for v2
- [ ] Future client preparation (Share, Stream, Mesh, Publish, Vault)

**Critical Path Items:**
- API compatibility shims for gradual migration
- Mobile client performance on constrained devices
- Feature parity between desktop and mobile

**Reference:** [Phase 5 Sprint Plan](v2-migration/phase-5-client-updates.md)

---

### Phase 6: Release (85-100 SP)

**Goal:** Complete documentation, changelog, and v3.0.0 release.

**Key Deliverables:**
- [ ] CHANGELOG.md v3.0.0 entry
- [ ] Migration guide updates
- [ ] API documentation refresh
- [ ] Security audit sign-off
- [ ] Performance certification
- [ ] v3.0.0 tag and release
- [ ] Compatibility mode deprecation timeline

**Critical Path Items:**
- All documentation synchronized
- Security review completed
- Performance benchmarks published

**Reference:** [Phase 6 Sprint Plan](v2-migration/phase-6-release.md)

---

## Performance Targets

### Network Performance

| Metric | v1 Baseline | v2 Target | Transport |
|--------|-------------|-----------|-----------|
| Throughput | 300 Mbps | 500 Mbps | UDP userspace |
| Throughput | 10 Gbps | 40 Gbps | AF_XDP |
| Throughput | N/A | 100 Gbps | AF_XDP optimized |
| Latency | <1ms | <500us | AF_XDP |
| Latency | 1-5ms | 1-3ms | UDP userspace |

### Cryptographic Performance

| Operation | Target | Notes |
|-----------|--------|-------|
| Hybrid KEM keygen | <1ms | X25519 + ML-KEM-768 |
| Hybrid encapsulate | <500us | Combined operation |
| Hybrid decapsulate | <500us | Combined operation |
| Per-packet ratchet | <1us | BLAKE3-based |
| AEAD encrypt | 10+ Gbps | XChaCha20-Poly1305 |
| AEAD decrypt | 10+ Gbps | XChaCha20-Poly1305 |

### Memory Targets

| Metric | Target |
|--------|--------|
| Session overhead | <100 KB |
| Per-stream overhead | <2 KB |
| Ratchet state | <512 bytes |
| Hybrid key pair | <5 KB |

### Scalability Targets

| Metric | Target |
|--------|--------|
| Concurrent sessions | 100,000 (single node) |
| Concurrent streams | 65,535 per session |
| Packet rate | 5M pps per core (AF_XDP) |
| Connection migration | <50ms handoff |

---

## Risk Assessment

### High Risk

| Risk | Impact | Mitigation |
|------|--------|------------|
| ML-KEM implementation bugs | Critical | Use audited `ml-kem` crate, extensive testing |
| Performance regression | High | Continuous benchmarking, fallback to v1 |
| Wire format incompatibility | High | Extensive interop testing, compat mode |
| Client migration delays | Medium | Parallel development, feature flags |

### Medium Risk

| Risk | Impact | Mitigation |
|------|--------|------------|
| MSRV upgrade (1.85) | Medium | Clear communication, staged rollout |
| Platform-specific issues | Medium | CI matrix expansion |
| Dependency breaking changes | Medium | Version pinning, careful updates |
| Documentation lag | Low | Dedicated documentation sprints |

### Low Risk

| Risk | Impact | Mitigation |
|------|--------|------------|
| Test coverage gaps | Low | Coverage enforcement, property testing |
| API ergonomics issues | Low | Early user feedback, shims |

---

## Success Criteria

### Technical Criteria

- [ ] All v2 cryptographic primitives implemented and tested
- [ ] Wire format encoding/decoding verified correct
- [ ] All performance targets met or exceeded
- [ ] Zero security vulnerabilities (audit clean)
- [ ] 95% test coverage on crypto, 80% overall
- [ ] All 9+ clients migrated and functional

### Process Criteria

- [ ] No regressions in v1 functionality
- [ ] Compatibility mode working for 90-day window
- [ ] Migration path documented and tested
- [ ] All breaking changes documented
- [ ] CHANGELOG complete and accurate

### Release Criteria

- [ ] v3.0.0 tagged and released
- [ ] All documentation updated
- [ ] Security audit completed
- [ ] Performance certification published
- [ ] Deprecation timeline announced

---

## Dependencies

### External Dependencies

| Dependency | Version | Purpose | Risk |
|------------|---------|---------|------|
| ml-kem | 0.2+ | Post-quantum KEM | Low (NIST standardized) |
| ml-dsa | 0.1+ | Post-quantum signatures | Low (NIST standardized) |
| blake3 | 1.5+ | Hashing, KDF | Very Low |
| tokio | 1.40+ | Async runtime | Very Low |
| snow | 0.9+ | Noise protocol | Very Low |

### Internal Dependencies

| Crate | Dependency Type | Notes |
|-------|----------------|-------|
| wraith-crypto | Foundation | Must complete Phase 1 first |
| wraith-core | Wire format | Depends on wraith-crypto |
| wraith-transport | Transport | Depends on wraith-core |
| wraith-obfuscation | Integration | Depends on wire format |
| wraith-cli | Client | Depends on all crates |
| Client apps | Final | Depend on wraith-ffi updates |

---

## Timeline (Estimated)

```
Week 1-3:   Phase 1 - Crypto Foundation
Week 4-5:   Phase 2 - Wire Format
Week 6-8:   Phase 3 - Transport
Week 9-11:  Phase 4 - Integration Testing
Week 12-15: Phase 5 - Client Updates
Week 16:    Phase 6 - Release

Total: 12-16 weeks
```

### Milestones

| Milestone | Target Week | Deliverable |
|-----------|-------------|-------------|
| M1 | Week 3 | Hybrid crypto complete |
| M2 | Week 5 | Wire format complete |
| M3 | Week 8 | Transport complete |
| M4 | Week 11 | Integration validated |
| M5 | Week 15 | Clients migrated |
| M6 | Week 16 | v3.0.0 released |

---

## Team Structure (Recommended)

| Role | Count | Focus |
|------|-------|-------|
| Crypto Lead | 1 | Phase 1, security review |
| Protocol Lead | 1 | Phases 2-4, integration |
| Client Lead | 1 | Phase 5, client apps |
| QA Lead | 1 | Testing, benchmarking |
| Documentation | 1 | Phase 6, docs |

---

## Related Documents

### Planning Documents

- [Phase 1: Crypto Foundation](v2-migration/phase-1-crypto-foundation.md)
- [Phase 2: Wire Format](v2-migration/phase-2-wire-format.md)
- [Phase 3: Transport](v2-migration/phase-3-transport.md)
- [Phase 4: Integration](v2-migration/phase-4-integration.md)
- [Phase 5: Client Updates](v2-migration/phase-5-client-updates.md)
- [Phase 6: Release](v2-migration/phase-6-release.md)
- [Technical Debt](../technical-debt/v2-migration-debt.md)

### Reference Documents

- [v2 Specification](../../ref-docs/v2_WRAITH-Protocol/01-WRAITH-Protocol-v2-Specification.md)
- [v2 Architecture](../../ref-docs/v2_WRAITH-Protocol/02-WRAITH-Protocol-v2-Architecture.md)
- [v1 to v2 Changelog](../../ref-docs/v2_WRAITH-Protocol/03-WRAITH-Protocol-v1-to-v2-Changelog.md)
- [Migration Guide](../../ref-docs/v2_WRAITH-Protocol/09-WRAITH-Protocol-v2-Migration-Guide.md)
- [Crypto Upgrades](../../ref-docs/v2_WRAITH-Protocol/12-WRAITH-Protocol-v2-Crypto-Upgrades.md)
- [Performance Targets](../../ref-docs/v2_WRAITH-Protocol/13-WRAITH-Protocol-v2-Performance-Targets.md)
- [Testing Strategy](../../ref-docs/v2_WRAITH-Protocol/14-WRAITH-Protocol-v2-Testing-Strategy.md)
- [API Changes](../../ref-docs/v2_WRAITH-Protocol/16-WRAITH-Protocol-v2-API-Changes.md)

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial master migration plan |

---

**Note:** This migration is planned for post-v2.3.0 (after WRAITH-RedOps client release). Current protocol version is v2.2.0.
