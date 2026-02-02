# Phase 3: Transport

**Parent:** [v2 Migration Master Plan](../v2-migration-master.md)
**Version:** 1.0.0
**Status:** Planning (Post-v2.3.0)
**Story Points:** 90-115 SP
**Duration:** 2-3 weeks
**Dependencies:** Phase 2 (Wire Format)

---

## Executive Summary

Phase 3 implements the multi-transport abstraction layer, connection migration, and cross-platform support. This enables WRAITH Protocol v2 to operate over UDP, TCP, WebSocket, QUIC, and HTTP/2/HTTP/3 transports with seamless migration between them.

### Objectives

1. Create unified async `Transport` trait
2. Implement `TransportManager` for multi-transport handling
3. Enable connection migration between transports
4. Add WebSocket and QUIC transport implementations
5. Ensure cross-platform support (Linux, macOS, Windows)

---

## Sprint Breakdown

### Sprint 3.1: Transport Trait Unification (16-20 SP)

**Goal:** Create unified async-only Transport trait.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 3.1.1 | Define async `Transport` trait | 3 | Critical | - |
| 3.1.2 | Add `transport_type()` method | 1 | Critical | - |
| 3.1.3 | Add `mtu()` method | 1 | High | - |
| 3.1.4 | Add `supports_migration()` method | 1 | High | - |
| 3.1.5 | Add `latency_estimate()` method | 2 | Medium | - |
| 3.1.6 | Add `stats()` method | 2 | Medium | - |
| 3.1.7 | Add `close()` method | 1 | Critical | - |
| 3.1.8 | Remove old sync `Transport` trait | 2 | High | - |
| 3.1.9 | Remove `AsyncTransport` (merged) | 1 | High | - |
| 3.1.10 | Update UDP transport to new trait | 3 | Critical | - |
| 3.1.11 | Update AF_XDP transport to new trait | 3 | Critical | - |
| 3.1.12 | Unit tests for trait compliance | 2 | Critical | - |

**Acceptance Criteria:**
- [ ] Single unified `Transport` trait
- [ ] All transports implement trait
- [ ] Async-first API (no sync methods in trait)
- [ ] Statistics available for all transports
- [ ] Migration capability queryable

**Transport Trait Definition:**
```rust
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

    /// Close transport gracefully
    async fn close(&self) -> Result<()>;
}
```

**Code Location:** `crates/wraith-transport/src/transport.rs`

---

### Sprint 3.2: TransportManager (21-26 SP)

**Goal:** Implement multi-transport management with fallback.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 3.2.1 | Define `TransportManager` struct | 3 | Critical | - |
| 3.2.2 | Implement transport registration | 2 | Critical | - |
| 3.2.3 | Implement primary transport selection | 3 | Critical | - |
| 3.2.4 | Implement `send()` with fallback | 5 | Critical | - |
| 3.2.5 | Implement `recv()` aggregation | 5 | Critical | - |
| 3.2.6 | Implement `migrate()` method | 5 | Critical | - |
| 3.2.7 | Implement `TransportSelector` trait | 3 | High | - |
| 3.2.8 | Default selector (latency-based) | 2 | High | - |
| 3.2.9 | Bandwidth-based selector | 2 | Medium | - |
| 3.2.10 | Manager statistics aggregation | 2 | Medium | - |
| 3.2.11 | Unit tests (multi-transport scenarios) | 3 | Critical | - |
| 3.2.12 | Integration tests (fallback behavior) | 3 | High | - |

**Acceptance Criteria:**
- [ ] Multiple transports can be registered
- [ ] Primary transport selectable
- [ ] Automatic fallback on transport failure
- [ ] Migration between transports works
- [ ] Statistics aggregated across transports

**TransportManager API:**
```rust
impl TransportManager {
    pub fn new(transports: Vec<Arc<dyn Transport>>) -> Self;
    pub async fn send(&self, data: &[u8]) -> Result<()>;
    pub async fn recv(&self, buf: &mut [u8]) -> Result<(usize, TransportType)>;
    pub async fn migrate(&self, target: TransportType) -> Result<()>;
    pub fn add_transport(&mut self, transport: Arc<dyn Transport>);
    pub fn remove_transport(&mut self, transport_type: TransportType);
    pub fn primary(&self) -> Arc<dyn Transport>;
    pub fn all_transports(&self) -> &[Arc<dyn Transport>];
    pub fn stats(&self) -> AggregatedStats;
}
```

**Code Location:** `crates/wraith-transport/src/manager.rs`

---

### Sprint 3.3: Connection Migration (18-23 SP)

**Goal:** Implement seamless connection migration between transports.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 3.3.1 | Define `MigrationState` state machine | 3 | Critical | - |
| 3.3.2 | Implement PATH_CHALLENGE frame | 2 | Critical | - |
| 3.3.3 | Implement PATH_RESPONSE frame | 2 | Critical | - |
| 3.3.4 | Implement PATH_MIGRATE frame | 2 | Critical | - |
| 3.3.5 | Migration initiation logic | 3 | Critical | - |
| 3.3.6 | Packet queueing during migration | 3 | Critical | - |
| 3.3.7 | Migration completion handshake | 3 | Critical | - |
| 3.3.8 | Migration failure recovery | 3 | High | - |
| 3.3.9 | Migration timeout handling | 2 | High | - |
| 3.3.10 | Unit tests (state machine) | 3 | Critical | - |
| 3.3.11 | Integration tests (full migration) | 5 | Critical | - |

**Acceptance Criteria:**
- [ ] Migration state machine correct
- [ ] PATH_* frames work correctly
- [ ] No packet loss during migration
- [ ] Migration completes within 50ms
- [ ] Failure recovery works

**Migration State Machine:**
```
                    ┌─────────────┐
                    │   Stable    │
                    └──────┬──────┘
                           │ initiate_migration()
                           ▼
                    ┌─────────────┐
          ┌────────│  Probing    │────────┐
          │        └──────┬──────┘        │
   timeout│               │ path_response │ failure
          │               ▼               │
          │        ┌─────────────┐        │
          │        │  Migrating  │        │
          │        └──────┬──────┘        │
          │               │ migrate_ack   │
          │               ▼               │
          │        ┌─────────────┐        │
          └───────►│   Stable    │◄───────┘
                   └─────────────┘
```

**Code Location:** `crates/wraith-transport/src/migration.rs`

---

### Sprint 3.4: WebSocket Transport (13-16 SP)

**Goal:** Implement WebSocket transport for firewall traversal.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 3.4.1 | Add `tokio-tungstenite` dependency | 1 | High | - |
| 3.4.2 | Define `WebSocketTransport` struct | 2 | High | - |
| 3.4.3 | Implement WebSocket connect | 3 | High | - |
| 3.4.4 | Implement `Transport` trait | 3 | High | - |
| 3.4.5 | Frame encapsulation in WS messages | 2 | High | - |
| 3.4.6 | Reconnection logic | 2 | Medium | - |
| 3.4.7 | TLS support | 2 | High | - |
| 3.4.8 | Unit tests | 2 | High | - |
| 3.4.9 | Integration tests (with real WS server) | 3 | Medium | - |

**Acceptance Criteria:**
- [ ] WebSocket connection established
- [ ] WRAITH frames transmitted over WS
- [ ] TLS encryption supported
- [ ] Reconnection on disconnect
- [ ] Works through HTTP proxies

**Code Location:** `crates/wraith-transport/src/websocket.rs`

---

### Sprint 3.5: QUIC Transport (16-20 SP)

**Goal:** Implement QUIC transport for modern deployments.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 3.5.1 | Add `quinn` dependency | 1 | High | - |
| 3.5.2 | Define `QuicTransport` struct | 2 | High | - |
| 3.5.3 | Implement QUIC client setup | 3 | High | - |
| 3.5.4 | Implement QUIC server setup | 3 | High | - |
| 3.5.5 | Implement `Transport` trait | 3 | High | - |
| 3.5.6 | 0-RTT resumption support | 3 | Medium | - |
| 3.5.7 | Connection migration (QUIC native) | 3 | High | - |
| 3.5.8 | Unit tests | 2 | High | - |
| 3.5.9 | Integration tests | 3 | High | - |

**Acceptance Criteria:**
- [ ] QUIC connection works
- [ ] Native QUIC migration utilized
- [ ] 0-RTT resumption reduces latency
- [ ] Certificate handling correct
- [ ] Congestion control interoperates

**Code Location:** `crates/wraith-transport/src/quic.rs`

---

### Sprint 3.6: HTTP/2 & HTTP/3 Transport (6-10 SP)

**Goal:** Implement HTTP/2 and HTTP/3 transports.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 3.6.1 | Define `Http2Transport` struct | 2 | Medium | - |
| 3.6.2 | Implement HTTP/2 streaming | 3 | Medium | - |
| 3.6.3 | Define `Http3Transport` struct | 2 | Medium | - |
| 3.6.4 | Implement HTTP/3 (via QUIC) | 3 | Medium | - |
| 3.6.5 | Unit tests | 2 | Medium | - |

**Acceptance Criteria:**
- [ ] HTTP/2 streaming works
- [ ] HTTP/3 via QUIC works
- [ ] Highly firewall-friendly
- [ ] CDN-compatible framing

**Code Location:** `crates/wraith-transport/src/http/`

---

## Technical Specifications

### Transport Type Enumeration

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransportType {
    // Core transports
    Udp,
    Tcp,
    AfXdp,
    IoUring,

    // Web transports
    WebSocket,
    WebSocketSecure,
    Http2,
    Http3,

    // Modern transports
    Quic,

    // Covert transports
    DnsCovert,
    IcmpCovert,
}
```

### Transport Statistics

```rust
pub struct TransportStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_dropped: u64,
    pub retransmissions: u64,
    pub rtt_estimate: Duration,
    pub bandwidth_estimate: u64,  // bytes/sec
    pub congestion_window: usize,
    pub last_activity: Instant,
}
```

### Platform Support Matrix

| Transport | Linux | macOS | Windows | Notes |
|-----------|-------|-------|---------|-------|
| UDP | Yes | Yes | Yes | Core transport |
| TCP | Yes | Yes | Yes | Fallback |
| AF_XDP | Yes | No | No | Linux 5.3+ |
| io_uring | Yes | No | No | Linux 5.1+ |
| WebSocket | Yes | Yes | Yes | All platforms |
| QUIC | Yes | Yes | Yes | Via quinn |
| HTTP/2 | Yes | Yes | Yes | All platforms |
| HTTP/3 | Yes | Yes | Yes | Via quinn |

---

## Testing Requirements

### Test Categories

| Category | Target Coverage | Method |
|----------|-----------------|--------|
| Unit Tests | 85% | Standard test framework |
| Integration | Multi-transport | Real network tests |
| Migration | End-to-end | Simulated scenarios |
| Platform | Per-platform | CI matrix |

### Test Cases

| Test Case | Description |
|-----------|-------------|
| T3.1 | Transport trait compliance for all transports |
| T3.2 | TransportManager fallback behavior |
| T3.3 | Migration state machine transitions |
| T3.4 | No packet loss during migration |
| T3.5 | WebSocket through HTTP proxy |
| T3.6 | QUIC 0-RTT resumption |
| T3.7 | Cross-platform transport behavior |

---

## Dependencies

### External Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| tokio-tungstenite | 0.21+ | WebSocket |
| quinn | 0.11+ | QUIC |
| hyper | 1.0+ | HTTP/2 |
| async-trait | 0.1+ | Async traits |

### Phase Dependencies

| Dependency | Type | Notes |
|------------|------|-------|
| Phase 2 | Required | Wire format for frames |
| Phase 1 | Required | Crypto for secure transports |

---

## Risk Assessment

### High Risk

| Risk | Mitigation |
|------|------------|
| Migration packet loss | Queue packets during migration |
| Platform fragmentation | Comprehensive CI matrix |
| Transport negotiation failures | Clear fallback hierarchy |

### Medium Risk

| Risk | Mitigation |
|------|------------|
| QUIC compatibility issues | Use well-tested quinn crate |
| WebSocket proxy issues | Test common proxy configurations |

---

## Deliverables Checklist

### Code Deliverables

- [ ] `crates/wraith-transport/src/transport.rs` - Unified trait
- [ ] `crates/wraith-transport/src/manager.rs` - TransportManager
- [ ] `crates/wraith-transport/src/migration.rs` - Connection migration
- [ ] `crates/wraith-transport/src/websocket.rs` - WebSocket transport
- [ ] `crates/wraith-transport/src/quic.rs` - QUIC transport
- [ ] `crates/wraith-transport/src/http/mod.rs` - HTTP transports

### Test Deliverables

- [ ] Unit tests for all modules
- [ ] Integration tests for multi-transport
- [ ] Migration tests
- [ ] Platform-specific tests

### Documentation Deliverables

- [ ] Transport API documentation
- [ ] Migration protocol specification
- [ ] Platform support matrix

---

## Gap Analysis (v2.3.7 Assessment)

### Current Implementation State

| Component | Status | Notes |
|-----------|--------|-------|
| Transport trait (async) | PARTIAL | `crates/wraith-transport/src/transport.rs` has async trait but missing `characteristics()`, `mtu()`, `close()` |
| UDP transport | COMPLETE | Both sync (`udp.rs`) and async (`udp_async.rs`) |
| AF_XDP | COMPLETE | `af_xdp.rs` with UMEM, ring buffers, batch ops, full stats |
| io_uring | COMPLETE | `io_uring.rs` for async file I/O |
| QUIC transport | STUB | `quic.rs` exists as placeholder |
| WebSocket mimicry | PARTIAL | `wraith-obfuscation/src/websocket_mimicry.rs` for framing, not transport |
| TLS mimicry | PARTIAL | `wraith-obfuscation/src/tls_mimicry.rs` for wrapping, not transport |
| DoH tunnel | COMPLETE | `wraith-obfuscation/src/doh_tunnel.rs` |
| Buffer pool | COMPLETE | `buffer_pool.rs` with NUMA-aware allocation |
| NUMA support | COMPLETE | `numa.rs` |
| Worker pools | COMPLETE | `worker.rs` |
| MTU discovery | COMPLETE | `mtu.rs` |
| Transport factory | COMPLETE | `factory.rs` |

### Gaps Identified

1. **Unified Transport trait**: Current trait is partial. Need to add `characteristics()`, `peer_addr()`, `close()` per v2 spec (doc 01, section 7.1). Estimated ~200 lines refactor.

2. **TransportManager**: Entirely missing. Multi-transport registration, fallback, selection. Estimated ~600 lines.

3. **WebSocket transport**: Mimicry framing exists in wraith-obfuscation but no standalone WebSocket transport impl. Need `tokio-tungstenite` integration. Estimated ~400 lines.

4. **QUIC transport**: Only a placeholder stub. Need `quinn` integration. Estimated ~500 lines.

5. **TCP transport**: Not present (v1 was UDP-only). Need basic TCP transport as fallback. Estimated ~200 lines.

6. **HTTP/2, HTTP/3 transports**: Not present. Medium priority per spec. Estimated ~600 lines total.

7. **Connection migration between transports**: PATH_CHALLENGE/PATH_RESPONSE exist in wraith-core frame types but no transport-level migration logic. Migration state machine needed. Estimated ~500 lines.

8. **Transport characteristics**: `TransportCharacteristics` struct (reliable, ordered, datagram_capable, base_latency, overhead, max_bandwidth) not implemented. Estimated ~150 lines.

9. **Cross-platform**: AF_XDP/io_uring are Linux-only (correct). Need graceful fallback on macOS/Windows. Current `#[cfg(target_os = "linux")]` gating is correct but TransportManager needs to handle platform differences.

### Inaccuracies

- Sprint 3.1 mentions "Remove old sync Transport trait" and "Remove AsyncTransport (merged)". Need to verify the current trait structure before planning removal.
- Sprint 3.6 (HTTP/2 & HTTP/3) at 6-10 SP seems underestimated for two transport implementations.

### Client Impact

Transport changes primarily affect:
- wraith-chat (depends on wraith-transport for real-time comms)
- wraith-android, wraith-ios (transport selection for mobile networks)
- wraith-redops team-server (wraith-transport for C2 channels)
- wraith-recon (wraith-transport for packet capture)

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial Phase 3 sprint plan |
| 1.1.0 | 2026-02-01 | Gap analysis with current implementation state |
