# v2 Migration Technical Debt

**Parent:** [v2 Migration Master Plan](../protocol/v2-migration-master.md)
**Version:** 1.0.0
**Status:** Planning (Post-v2.3.0)
**Last Updated:** 2026-01-24

---

## Executive Summary

This document tracks technical debt items that may arise during the v1 to v2 migration, along with known technical compromises, deferred work, and items requiring future attention.

### Technical Debt Categories

| Category | Count | Priority |
|----------|-------|----------|
| Migration Shortcuts | 8 | Medium |
| Compatibility Shims | 5 | High |
| Deferred Optimizations | 6 | Low |
| Documentation Gaps | 4 | Medium |
| Test Coverage Gaps | 5 | High |
| **Total** | **28** | |

---

## Migration Shortcuts

### MS-001: v1 Connection ID Zero-Extension

**Status:** Planned
**Phase:** 2 (Wire Format)
**Priority:** Medium
**Story Points:** 3

**Description:**
v1 64-bit ConnectionIds are migrated to v2 by zero-extending to 128 bits. This creates a detectable pattern where migrated CIDs have all zeros in the upper 64 bits.

**Current Implementation:**
```rust
impl ConnectionId {
    pub fn from_v1(v1_cid: u64) -> Self {
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&v1_cid.to_le_bytes());
        // Upper 8 bytes are zeros - detectable pattern
        Self { bytes }
    }
}
```

**Ideal Solution:**
Derive the full 128-bit CID from the v1 CID using a PRF to prevent pattern detection:
```rust
fn from_v1_secure(v1_cid: u64, session_secret: &[u8; 32]) -> Self {
    let mut bytes = [0u8; 16];
    let hash = blake3::keyed_hash(b"wraith-v2-cid-expand", session_secret);
    bytes[0..8].copy_from_slice(&v1_cid.to_le_bytes());
    bytes[8..16].copy_from_slice(&hash.as_bytes()[0..8]);
    Self { bytes }
}
```

**Resolution Timeline:** v3.1.0 (if traffic analysis concern materializes)

---

### MS-002: Sync API Wrapper Performance

**Status:** Planned
**Phase:** 5 (Client Updates)
**Priority:** Low
**Story Points:** 2

**Description:**
The `send_blocking()` wrapper for synchronous callers spawns a Tokio runtime per call, causing overhead for high-frequency sync operations.

**Current Implementation:**
```rust
pub fn send_blocking(&self, data: &[u8]) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(self.send(data))
}
```

**Ideal Solution:**
Maintain a shared runtime for blocking operations:
```rust
lazy_static! {
    static ref BLOCKING_RUNTIME: Runtime = Runtime::new().unwrap();
}

pub fn send_blocking(&self, data: &[u8]) -> Result<()> {
    BLOCKING_RUNTIME.block_on(self.send(data))
}
```

**Resolution Timeline:** v3.1.0 (if performance issues reported)

---

### MS-003: Polymorphic Format Lookup Tables

**Status:** Planned
**Phase:** 2 (Wire Format)
**Priority:** Low
**Story Points:** 5

**Description:**
Polymorphic format field position computation is done at runtime rather than using precomputed lookup tables, adding latency to every packet.

**Current Implementation:**
Compute field positions on each encode/decode operation.

**Ideal Solution:**
Precompute position lookup tables when format is derived from session secret, amortizing computation across all packets.

**Resolution Timeline:** v3.1.0 (optimization sprint)

---

### MS-004: ML-KEM Key Generation Blocking

**Status:** Planned
**Phase:** 1 (Crypto Foundation)
**Priority:** Medium
**Story Points:** 3

**Description:**
ML-KEM-768 key generation takes ~1ms which may block async runtimes if called from async context without spawn_blocking.

**Mitigation:**
Document that key generation should use `tokio::task::spawn_blocking()` in async contexts.

**Ideal Solution:**
Provide both sync and async key generation APIs with async version using spawn_blocking internally.

**Resolution Timeline:** v3.0.0 (document as known limitation)

---

### MS-005: HKDF Label String Allocations

**Status:** Planned
**Phase:** 1 (Crypto Foundation)
**Priority:** Low
**Story Points:** 2

**Description:**
HKDF label construction allocates strings at runtime rather than using compile-time constants.

**Current Implementation:**
```rust
let label = format!("wraith-v2-{}", context);
hkdf_blake3(secret, label.as_bytes(), output_len)
```

**Ideal Solution:**
Use `const` byte arrays for all labels:
```rust
const LABEL_HANDSHAKE_INIT: &[u8] = b"wraith-v2-handshake-init";
hkdf_blake3(secret, LABEL_HANDSHAKE_INIT, output_len)
```

**Resolution Timeline:** v3.0.0 (address during implementation)

---

### MS-006: Transport Fallback Chain Hardcoded

**Status:** Planned
**Phase:** 3 (Transport)
**Priority:** Medium
**Story Points:** 3

**Description:**
TransportManager fallback order is hardcoded rather than configurable per-deployment.

**Current Plan:**
Hardcode sensible defaults (QUIC > UDP > WebSocket > TCP).

**Ideal Solution:**
Make fallback chain configurable via session builder or config file.

**Resolution Timeline:** v3.1.0

---

### MS-007: Migration State Machine Not Cancellation-Safe

**Status:** Planned
**Phase:** 3 (Transport)
**Priority:** High
**Story Points:** 5

**Description:**
Connection migration state machine may leave orphaned state if cancelled mid-migration.

**Mitigation:**
Document that migration should not be cancelled, implement cleanup on drop.

**Ideal Solution:**
Design state machine to be fully cancellation-safe with automatic cleanup.

**Resolution Timeline:** v3.0.0 (implement cleanup on drop)

---

### MS-008: Client Feature Parity Incomplete

**Status:** Planned
**Phase:** 5 (Client Updates)
**Priority:** Medium
**Story Points:** 8

**Description:**
Not all v2 features will be exposed in all clients at v3.0.0 release. Mobile clients may have limited multi-stream support.

**Affected Features:**
- Multi-stream (mobile: limited to 4 streams)
- Transport migration (mobile: UDP/WebSocket only)
- Group communication (desktop only at v3.0.0)

**Resolution Timeline:** v3.1.0 - v3.2.0 (incremental client updates)

---

## Compatibility Shims

### CS-001: Session::new Deprecated Wrapper

**Status:** Planned
**Phase:** 5 (Client Updates)
**Priority:** High
**Story Points:** 2

**Description:**
The deprecated `Session::new()` constructor wraps the builder pattern, adding overhead and preventing access to v2 features.

**Shim Code:**
```rust
#[deprecated(since = "3.0.0", note = "Use Session::builder()")]
pub fn new(cid: ConnectionId, addr: SocketAddr, secret: [u8; 32]) -> Result<Self> {
    Self::builder()
        .connection_id(cid)
        .peer_addr(addr)
        .crypto_context(CryptoContext::from_classical(secret))
        .v1_compat(true)  // Forces v1 behavior
        .build()
}
```

**Debt:** Users of this API are locked into classical crypto and v1 compat mode.

**Resolution:** Remove in v4.0.0

---

### CS-002: 64-bit ConnectionId Type Alias

**Status:** Planned
**Phase:** 2 (Wire Format)
**Priority:** High
**Story Points:** 1

**Description:**
Type alias maintains 64-bit CID for API compatibility.

**Shim Code:**
```rust
#[deprecated(since = "3.0.0", note = "Use ConnectionId (128-bit)")]
pub type ConnectionId64 = u64;
```

**Resolution:** Remove in v4.0.0

---

### CS-003: hkdf_sha256 Deprecated Function

**Status:** Planned
**Phase:** 1 (Crypto Foundation)
**Priority:** High
**Story Points:** 1

**Description:**
SHA256 KDF retained for compatibility but should not be used for new code.

**Resolution:** Remove in v4.0.0

---

### CS-004: Fixed Padding Classes

**Status:** Planned
**Phase:** 2 (Wire Format)
**Priority:** Medium
**Story Points:** 2

**Description:**
Fixed padding classes (64, 128, 256, etc.) retained but continuous distribution is preferred.

**Resolution:** Remove in v4.0.0

---

### CS-005: Sync Transport Trait

**Status:** Planned
**Phase:** 3 (Transport)
**Priority:** High
**Story Points:** 3

**Description:**
Sync transport trait wrapper for legacy code that hasn't migrated to async.

**Resolution:** Remove in v4.0.0

---

## Deferred Optimizations

### DO-001: SIMD Polymorphic Encoding

**Status:** Deferred
**Phase:** 2 (Wire Format)
**Priority:** Low
**Story Points:** 8

**Description:**
Polymorphic header encoding/decoding could benefit from SIMD but is not in initial scope.

**Impact:** ~10-20% potential speedup on high-throughput workloads.

**Resolution Timeline:** Post-v3.0.0 optimization sprint

---

### DO-002: Zero-Copy Multi-Stream

**Status:** Deferred
**Phase:** 3 (Transport)
**Priority:** Low
**Story Points:** 13

**Description:**
Multi-stream implementation copies data between streams rather than using zero-copy techniques.

**Impact:** Higher memory usage and CPU for high-stream-count sessions.

**Resolution Timeline:** v3.2.0 (if performance issues reported)

---

### DO-003: Ratchet Key Cache Lock-Free

**Status:** Deferred
**Phase:** 1 (Crypto Foundation)
**Priority:** Low
**Story Points:** 8

**Description:**
Per-packet ratchet key cache uses mutex rather than lock-free data structure.

**Impact:** Potential contention under extreme packet rates.

**Resolution Timeline:** Post-v3.0.0 (if contention observed)

---

### DO-004: ML-KEM AVX-512 Optimization

**Status:** Deferred
**Phase:** 1 (Crypto Foundation)
**Priority:** Low
**Story Points:** 13

**Description:**
ML-KEM operations could use AVX-512 on supported CPUs for ~30% speedup.

**Impact:** Missed optimization on server-class hardware.

**Resolution Timeline:** Depends on upstream `ml-kem` crate support

---

### DO-005: Transport Statistics Lock-Free

**Status:** Deferred
**Phase:** 3 (Transport)
**Priority:** Low
**Story Points:** 5

**Description:**
Transport statistics use atomic counters but aggregation requires locking.

**Resolution Timeline:** v3.1.0

---

### DO-006: Connection Pool Pre-allocation

**Status:** Deferred
**Phase:** 3 (Transport)
**Priority:** Low
**Story Points:** 5

**Description:**
Connection pools allocate on demand rather than pre-allocating for expected capacity.

**Resolution Timeline:** v3.1.0 (if allocation overhead measured)

---

## Documentation Gaps

### DG-001: Polymorphic Format Internals

**Status:** Planned
**Phase:** 6 (Release)
**Priority:** Medium
**Story Points:** 3

**Description:**
Internal documentation of polymorphic format derivation algorithm is incomplete.

**Resolution:** Complete before v3.0.0

---

### DG-002: Migration Decision Tree

**Status:** Planned
**Phase:** 6 (Release)
**Priority:** Medium
**Story Points:** 2

**Description:**
Clear decision tree for when to use v1 compat vs native v2 is missing.

**Resolution:** Complete before v3.0.0

---

### DG-003: Performance Tuning Guide

**Status:** Planned
**Phase:** 6 (Release)
**Priority:** Low
**Story Points:** 5

**Description:**
Guide for tuning v2 performance parameters (ratchet cache size, transport selection, etc.) not written.

**Resolution:** v3.1.0

---

### DG-004: Mobile-Specific Limitations

**Status:** Planned
**Phase:** 5 (Client Updates)
**Priority:** Medium
**Story Points:** 2

**Description:**
Documentation of mobile-specific v2 limitations and workarounds incomplete.

**Resolution:** Complete before v3.0.0

---

## Test Coverage Gaps

### TC-001: Polymorphic Format Fuzzing

**Status:** Planned
**Phase:** 4 (Integration)
**Priority:** High
**Story Points:** 5

**Description:**
Polymorphic format parser needs extensive fuzzing to ensure robustness.

**Resolution:** Required for v3.0.0

---

### TC-002: Migration State Machine Coverage

**Status:** Planned
**Phase:** 4 (Integration)
**Priority:** High
**Story Points:** 5

**Description:**
Connection migration state machine has complex edge cases that need property-based testing.

**Resolution:** Required for v3.0.0

---

### TC-003: Cross-Platform Transport Tests

**Status:** Planned
**Phase:** 4 (Integration)
**Priority:** High
**Story Points:** 8

**Description:**
Transport layer tests need expansion for Windows and macOS specific behaviors.

**Resolution:** Required for v3.0.0

---

### TC-004: Mobile Client E2E Tests

**Status:** Planned
**Phase:** 5 (Client Updates)
**Priority:** High
**Story Points:** 8

**Description:**
End-to-end tests for mobile clients on real devices are limited.

**Resolution:** v3.0.0 (minimum viable), v3.1.0 (comprehensive)

---

### TC-005: Group Communication Tests

**Status:** Deferred
**Phase:** Post-v3.0.0
**Priority:** Medium
**Story Points:** 13

**Description:**
Group communication (TreeKEM) needs extensive testing but is not in v3.0.0 scope.

**Resolution:** v3.1.0 (when group features enabled)

---

## Debt Metrics

### Summary by Priority

| Priority | Items | Story Points |
|----------|-------|--------------|
| High | 11 | 48 |
| Medium | 10 | 41 |
| Low | 7 | 59 |
| **Total** | **28** | **148** |

### Summary by Phase

| Phase | Items | Story Points |
|-------|-------|--------------|
| Phase 1 (Crypto) | 5 | 27 |
| Phase 2 (Wire Format) | 5 | 21 |
| Phase 3 (Transport) | 6 | 39 |
| Phase 4 (Integration) | 3 | 18 |
| Phase 5 (Clients) | 5 | 23 |
| Phase 6 (Release) | 2 | 5 |
| Post-v3.0.0 | 2 | 15 |
| **Total** | **28** | **148** |

### Resolution Timeline

| Release | Items Resolved | Story Points |
|---------|----------------|--------------|
| v3.0.0 | 15 | 75 |
| v3.1.0 | 8 | 43 |
| v3.2.0 | 3 | 18 |
| v4.0.0 | 2 | 12 |

---

## Debt Reduction Strategy

### v3.0.0 Focus

1. **Address all High priority items** - Security and functionality critical
2. **Complete documentation** - Required for release
3. **Test coverage minimums** - 80% overall, 95% crypto

### v3.1.0 Focus

1. **Optimization sprint** - Address deferred optimizations
2. **Mobile feature parity** - Complete mobile client features
3. **Performance tuning guide** - Documentation completion

### v4.0.0 Focus

1. **Remove all compatibility shims** - Clean API
2. **Group communication completion** - Full TreeKEM support
3. **Zero remaining high-priority debt**

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial technical debt document |
