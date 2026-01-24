# WRAITH Protocol v2 Implementation Phases

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Planning Document
**Authors:** WRAITH Protocol Team

---

## Table of Contents

1. [Overview](#overview)
2. [Phase Summary](#phase-summary)
3. [Phase 1: Foundation](#phase-1-foundation)
4. [Phase 2: Core Protocol](#phase-2-core-protocol)
5. [Phase 3: Transport Layer](#phase-3-transport-layer)
6. [Phase 4: Security Hardening](#phase-4-security-hardening)
7. [Phase 5: Production Readiness](#phase-5-production-readiness)
8. [Resource Requirements](#resource-requirements)
9. [Risk Assessment](#risk-assessment)
10. [Success Criteria](#success-criteria)

---

## Overview

This document details the phased implementation plan for WRAITH Protocol v2. The implementation spans 44 weeks across 5 major phases, requiring a team of 12 FTE with specialized expertise in cryptography, systems programming, and security.

### Timeline Overview

```
Week:  1    8    16   24   32   40   44
       │    │    │    │    │    │    │
       ├────┼────┼────┼────┼────┼────┤
       │ P1 │ P2      │ P3      │P4 │P5│
       │    │         │         │   │  │
       └────┴─────────┴─────────┴───┴──┘

P1: Foundation (8 weeks)
P2: Core Protocol (16 weeks)
P3: Transport Layer (16 weeks)
P4: Security Hardening (8 weeks)
P5: Production Readiness (4 weeks, overlaps P4)
```

---

## Phase Summary

| Phase | Duration | Focus | Key Deliverables |
|-------|----------|-------|------------------|
| 1 | Weeks 1-8 | Foundation | Crypto primitives, wire format |
| 2 | Weeks 9-24 | Core Protocol | Handshake, sessions, frames |
| 3 | Weeks 17-32 | Transport | Multi-transport, kernel bypass |
| 4 | Weeks 33-40 | Security | Audit, hardening, PQ integration |
| 5 | Weeks 37-44 | Production | Testing, docs, release prep |

### Parallel Work Streams

```
        Week 1      Week 8      Week 16     Week 24     Week 32     Week 40
          │           │           │           │           │           │
Stream 1: ├───[Crypto Primitives]─┼───[Hybrid PQ]────────┼───[Audit]──┤
          │                       │                      │            │
Stream 2: ├───[Wire Format]───────┼───[Polymorphic]──────┤            │
          │                       │                      │            │
Stream 3:           ├───[Handshake Protocol]─────────────┤            │
          │                       │                      │            │
Stream 4:                    ├───[Transport Layer]───────┼───[Opt]────┤
          │                       │                      │            │
Stream 5:                              ├───[Obfuscation]─┼───[ML]─────┤
          │                       │                      │            │
Stream 6:                                        ├───[Integration]────┤
```

---

## Phase 1: Foundation

**Duration:** Weeks 1-8 (8 weeks)
**Team Size:** 4 FTE
**Story Points:** ~200

### Objectives

1. Establish cryptographic foundation with hybrid PQ support
2. Design and implement v2 wire format
3. Create compatibility layer for v1
4. Set up CI/CD and testing infrastructure

### Sprint Breakdown

#### Sprint 1.1: Cryptographic Primitives (Weeks 1-2)

**Deliverables:**
- ML-KEM-768 implementation (FIPS 203 compliant)
- ML-DSA-65 implementation (FIPS 204 compliant)
- BLAKE3-based key derivation
- Hybrid key combination logic

```rust
// Key milestone: Hybrid KEM
pub struct HybridKem {
    classical: X25519,
    post_quantum: MlKem768,
}

impl HybridKem {
    pub fn encapsulate(
        &self,
        peer_pk: &HybridPublicKey,
    ) -> Result<(SharedSecret, HybridCiphertext)> {
        let (ss1, ct1) = self.classical.encapsulate(&peer_pk.classical)?;
        let (ss2, ct2) = self.post_quantum.encapsulate(&peer_pk.pq)?;

        let combined = blake3::keyed_hash(
            b"wraith-hybrid-kem-v2-ss-combine",
            &[ss1.as_bytes(), ss2.as_bytes()].concat(),
        );

        Ok((SharedSecret::from(combined), HybridCiphertext { ct1, ct2 }))
    }
}
```

**Acceptance Criteria:**
- [ ] All NIST test vectors pass
- [ ] Performance within 2x of pure classical
- [ ] Memory-safe implementation verified

#### Sprint 1.2: Wire Format v2 (Weeks 3-4)

**Deliverables:**
- 24-byte frame header implementation
- 128-bit connection ID support
- Version negotiation protocol
- Polymorphic format foundation

```rust
// Key milestone: v2 Frame Header
#[repr(C)]
pub struct FrameHeaderV2 {
    pub version: u8,           // Protocol version (0x02)
    pub frame_type: u8,        // Frame type enum
    pub flags: u8,             // Frame flags
    pub reserved: u8,          // Reserved for future use
    pub sequence: u64,         // Packet sequence number
    pub length: u32,           // Payload length
    pub connection_id: u128,   // 128-bit connection ID
}

impl FrameHeaderV2 {
    pub const SIZE: usize = 24;

    pub fn encode(&self, format_key: &FormatKey) -> [u8; Self::SIZE] {
        let mut header = [0u8; Self::SIZE];
        // Polymorphic encoding based on format_key
        self.encode_polymorphic(&mut header, format_key);
        header
    }
}
```

**Acceptance Criteria:**
- [ ] Header parsing/serialization works
- [ ] Polymorphic encoding produces unique patterns
- [ ] Backward compatible with v1 detection

#### Sprint 1.3: Compatibility Layer (Weeks 5-6)

**Deliverables:**
- v1 protocol detection
- Version negotiation handshake
- Dual-mode session support
- Migration utilities

```rust
// Key milestone: Version Negotiation
pub async fn negotiate_version<T: AsyncReadWrite>(
    stream: &mut T,
    supported: VersionSet,
    role: Role,
) -> Result<NegotiatedVersion> {
    match role {
        Role::Initiator => {
            // Send version probe
            let probe = VersionProbe::new(supported);
            stream.write_all(&probe.encode()).await?;

            // Receive selection
            let response = VersionResponse::read(stream).await?;
            validate_selection(response.selected, supported)?;
            Ok(response.selected.into())
        }
        Role::Responder => {
            // Receive probe
            let probe = VersionProbe::read(stream).await?;

            // Select highest common version
            let selected = select_version(probe.supported, supported)?;
            let response = VersionResponse::new(selected);
            stream.write_all(&response.encode()).await?;
            Ok(selected.into())
        }
    }
}
```

**Acceptance Criteria:**
- [ ] v1 clients can connect to v2 servers
- [ ] v2 clients can connect to v1 servers
- [ ] Seamless version upgrade during session

#### Sprint 1.4: Infrastructure (Weeks 7-8)

**Deliverables:**
- CI/CD pipeline for v2
- Comprehensive test harness
- Benchmarking framework
- Documentation tooling

**Acceptance Criteria:**
- [ ] All tests run in CI
- [ ] Code coverage > 80%
- [ ] Automated security scanning
- [ ] API documentation generated

### Phase 1 Exit Criteria

- [ ] Hybrid cryptography passes all test vectors
- [ ] Wire format v2 fully specified and implemented
- [ ] Compatibility layer functional
- [ ] CI/CD operational

---

## Phase 2: Core Protocol

**Duration:** Weeks 9-24 (16 weeks)
**Team Size:** 6 FTE
**Story Points:** ~400

### Objectives

1. Implement hybrid handshake protocol
2. Build session management with per-packet ratcheting
3. Develop polymorphic wire format
4. Create stream multiplexing layer

### Sprint Breakdown

#### Sprint 2.1: Hybrid Handshake (Weeks 9-12)

**Deliverables:**
- Noise_XX + ML-KEM hybrid pattern
- Identity binding with PQ signatures
- Probing resistance mechanism
- Retry and resumption logic

```rust
// Key milestone: Hybrid Handshake
pub struct HybridHandshake {
    noise_state: snow::HandshakeState,
    pq_keypair: MlKem768KeyPair,
    role: HandshakeRole,
}

impl HybridHandshake {
    pub async fn execute<T: AsyncReadWrite>(
        &mut self,
        stream: &mut T,
    ) -> Result<SessionSecrets> {
        // Phase 1: Noise_XX with X25519
        let noise_output = self.execute_noise(stream).await?;

        // Phase 2: ML-KEM encapsulation
        let pq_output = self.execute_pq_kem(stream).await?;

        // Combine secrets
        let combined = combine_secrets(
            &noise_output.shared_secret,
            &pq_output.shared_secret,
        )?;

        // Derive session keys
        let secrets = derive_session_secrets(&combined)?;

        Ok(secrets)
    }
}
```

**Acceptance Criteria:**
- [ ] Handshake completes in < 2 RTT
- [ ] Probing resistance verified
- [ ] Forward secrecy maintained
- [ ] All handshake tests pass

#### Sprint 2.2: Session Management (Weeks 13-16)

**Deliverables:**
- Per-packet forward secrecy ratchet
- Session state machine
- Connection migration support
- Multi-path session handling

```rust
// Key milestone: Per-Packet Ratchet
pub struct PacketRatchet {
    chain_key: ChainKey,
    packet_number: u64,
    window: SlidingWindow,
}

impl PacketRatchet {
    pub fn advance(&mut self) -> MessageKey {
        let msg_key = blake3::keyed_hash(
            b"wraith-v2-message-key",
            self.chain_key.as_bytes(),
        );

        self.chain_key = ChainKey::from(blake3::keyed_hash(
            b"wraith-v2-chain-advance",
            self.chain_key.as_bytes(),
        ));

        self.packet_number += 1;
        MessageKey::from(msg_key)
    }

    pub fn derive_for_packet(&mut self, pn: u64) -> Result<MessageKey> {
        if pn < self.packet_number {
            return Err(Error::PacketReplay);
        }

        // Advance to target packet
        while self.packet_number < pn {
            let _ = self.advance();
        }

        Ok(self.advance())
    }
}
```

**Acceptance Criteria:**
- [ ] Per-packet ratchet operational
- [ ] Connection migration works
- [ ] Session resumption functional
- [ ] State machine fully tested

#### Sprint 2.3: Polymorphic Wire Format (Weeks 17-20)

**Deliverables:**
- Session-derived format keys
- Format transformation engine
- Multiple format profiles
- Format negotiation

```rust
// Key milestone: Polymorphic Format
pub struct PolymorphicFormat {
    format_key: FormatKey,
    field_positions: FieldPositions,
    field_sizes: FieldSizes,
    obfuscation_mask: [u8; 32],
}

impl PolymorphicFormat {
    pub fn derive(session_secret: &SessionSecret) -> Self {
        let format_key = blake3::keyed_hash(
            b"wraith-v2-format-key",
            session_secret.as_bytes(),
        );

        let positions = derive_field_positions(&format_key);
        let sizes = derive_field_sizes(&format_key);
        let mask = derive_obfuscation_mask(&format_key);

        Self {
            format_key: FormatKey::from(format_key),
            field_positions: positions,
            field_sizes: sizes,
            obfuscation_mask: mask,
        }
    }

    pub fn encode_header(&self, header: &FrameHeader) -> Vec<u8> {
        let mut encoded = vec![0u8; self.header_size()];

        // Place fields at derived positions
        self.encode_field(&mut encoded, Field::Version, header.version);
        self.encode_field(&mut encoded, Field::Type, header.frame_type);
        self.encode_field(&mut encoded, Field::Sequence, header.sequence);
        self.encode_field(&mut encoded, Field::Length, header.length);
        self.encode_field(&mut encoded, Field::ConnectionId, header.cid);

        // Apply obfuscation
        self.apply_mask(&mut encoded);

        encoded
    }
}
```

**Acceptance Criteria:**
- [ ] Format varies per session
- [ ] No static fingerprint possible
- [ ] Performance overhead < 5%
- [ ] All format tests pass

#### Sprint 2.4: Stream Multiplexing (Weeks 21-24)

**Deliverables:**
- Multi-stream support
- Priority scheduling
- Flow control per stream
- Stream migration

**Acceptance Criteria:**
- [ ] 1000+ concurrent streams supported
- [ ] Priority scheduling works correctly
- [ ] Flow control prevents HOL blocking
- [ ] Stream APIs documented

### Phase 2 Exit Criteria

- [ ] Hybrid handshake operational
- [ ] Per-packet ratcheting verified
- [ ] Polymorphic format fingerprint-resistant
- [ ] Stream multiplexing complete

---

## Phase 3: Transport Layer

**Duration:** Weeks 17-32 (16 weeks, overlaps Phase 2)
**Team Size:** 4 FTE
**Story Points:** ~350

### Objectives

1. Implement multi-transport abstraction
2. Add kernel bypass paths (AF_XDP, io_uring)
3. Enable transport migration
4. Optimize for performance targets

### Sprint Breakdown

#### Sprint 3.1: Transport Abstraction (Weeks 17-20)

**Deliverables:**
- Transport trait definition
- UDP transport (baseline)
- TCP transport
- WebSocket transport

```rust
// Key milestone: Transport Trait
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Send data over this transport
    async fn send(&self, data: &[u8]) -> Result<()>;

    /// Receive data from this transport
    async fn recv(&self, buf: &mut [u8]) -> Result<usize>;

    /// Get transport type
    fn transport_type(&self) -> TransportType;

    /// Check if transport supports connection migration
    fn supports_migration(&self) -> bool;

    /// Get current MTU
    fn mtu(&self) -> usize;

    /// Get latency estimate
    fn latency_estimate(&self) -> Duration;

    /// Transport-specific statistics
    fn stats(&self) -> TransportStats;
}

// Transport manager
pub struct TransportManager {
    transports: Vec<Arc<dyn Transport>>,
    primary: usize,
    selector: TransportSelector,
}

impl TransportManager {
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let transport = self.select_transport(data.len())?;
        transport.send(data).await
    }

    pub async fn migrate(&mut self, to: TransportType) -> Result<()> {
        let idx = self.find_transport(to)?;
        self.primary = idx;
        Ok(())
    }
}
```

**Acceptance Criteria:**
- [ ] All transport types implemented
- [ ] Abstraction hides transport details
- [ ] Migration between transports works
- [ ] Performance baseline established

#### Sprint 3.2: Kernel Bypass - AF_XDP (Weeks 21-24)

**Deliverables:**
- AF_XDP socket implementation
- UMEM management
- Zero-copy packet path
- XDP program (eBPF)

```rust
// Key milestone: AF_XDP Socket
pub struct AfXdpSocket {
    umem: Arc<Umem>,
    rx_ring: RxRing,
    tx_ring: TxRing,
    fill_ring: FillRing,
    completion_ring: CompletionRing,
    xsk_fd: RawFd,
}

impl AfXdpSocket {
    pub fn new(config: AfXdpConfig) -> Result<Self> {
        // Allocate UMEM
        let umem = Umem::new(config.umem_size, config.frame_size)?;

        // Create XSK socket
        let xsk_fd = unsafe {
            libc::socket(
                libc::AF_XDP,
                libc::SOCK_RAW,
                0,
            )
        };

        // Setup rings
        let rings = setup_rings(xsk_fd, &umem, &config)?;

        Ok(Self {
            umem: Arc::new(umem),
            rx_ring: rings.rx,
            tx_ring: rings.tx,
            fill_ring: rings.fill,
            completion_ring: rings.completion,
            xsk_fd,
        })
    }

    pub fn recv_batch(&mut self, batch: &mut [PacketBuffer]) -> usize {
        let available = self.rx_ring.available();
        let count = available.min(batch.len());

        for i in 0..count {
            let desc = self.rx_ring.consume();
            batch[i] = self.umem.get_packet(desc);
        }

        // Refill fill ring
        self.refill_fill_ring(count);

        count
    }
}
```

**Acceptance Criteria:**
- [ ] AF_XDP functional on Linux 6.2+
- [ ] Zero-copy path verified
- [ ] Throughput > 10 Gbps
- [ ] Latency < 10 microseconds

#### Sprint 3.3: Kernel Bypass - io_uring (Weeks 25-28)

**Deliverables:**
- io_uring integration
- Async file I/O
- Network I/O (where supported)
- Batch submission/completion

```rust
// Key milestone: io_uring Integration
pub struct IoUringTransport {
    ring: IoUring,
    pending_ops: HashMap<u64, PendingOp>,
    next_id: AtomicU64,
}

impl IoUringTransport {
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        unsafe {
            let sqe = self.ring.submission()
                .get_sqe()
                .ok_or(Error::QueueFull)?;

            sqe.prep_send(self.socket_fd, data, 0);
            sqe.set_user_data(id);
        }

        self.ring.submit()?;
        self.wait_completion(id).await
    }

    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        unsafe {
            let sqe = self.ring.submission()
                .get_sqe()
                .ok_or(Error::QueueFull)?;

            sqe.prep_recv(self.socket_fd, buf, 0);
            sqe.set_user_data(id);
        }

        self.ring.submit()?;
        self.wait_completion(id).await
    }
}
```

**Acceptance Criteria:**
- [ ] io_uring works for file I/O
- [ ] Network I/O on supported kernels
- [ ] Batch operations efficient
- [ ] Integration with transport manager

#### Sprint 3.4: QUIC Transport (Weeks 29-32)

**Deliverables:**
- QUIC transport implementation
- Connection migration via QUIC
- 0-RTT resumption
- HTTP/3 tunneling option

**Acceptance Criteria:**
- [ ] QUIC transport functional
- [ ] Migration works seamlessly
- [ ] 0-RTT provides performance benefit
- [ ] Interop with standard QUIC implementations

### Phase 3 Exit Criteria

- [ ] All transport types operational
- [ ] Kernel bypass paths working
- [ ] Transport migration seamless
- [ ] Performance targets met

---

## Phase 4: Security Hardening

**Duration:** Weeks 33-40 (8 weeks)
**Team Size:** 4 FTE (2 security specialists)
**Story Points:** ~200

### Objectives

1. Comprehensive security audit
2. Probing resistance verification
3. Post-quantum security validation
4. Timing attack mitigation

### Sprint Breakdown

#### Sprint 4.1: Internal Security Audit (Weeks 33-34)

**Deliverables:**
- Code review of all crypto paths
- Fuzzing campaign
- Static analysis
- Dependency audit

```rust
// Key milestone: Fuzzing Harness
#[cfg(fuzzing)]
pub mod fuzz_targets {
    use super::*;
    use arbitrary::Arbitrary;

    #[derive(Arbitrary)]
    pub struct FuzzInput {
        pub data: Vec<u8>,
        pub session_key: [u8; 32],
    }

    pub fn fuzz_frame_parse(input: &FuzzInput) {
        let format = PolymorphicFormat::derive(
            &SessionSecret::from(input.session_key)
        );

        // Should not panic on any input
        let _ = format.decode_header(&input.data);
    }

    pub fn fuzz_handshake(input: &[u8]) {
        let mut cursor = std::io::Cursor::new(input);
        let _ = HandshakeMessage::parse(&mut cursor);
    }
}
```

**Acceptance Criteria:**
- [ ] No critical vulnerabilities found
- [ ] Fuzzing coverage > 90%
- [ ] All dependencies audited
- [ ] SAST tools pass

#### Sprint 4.2: External Security Audit (Weeks 35-36)

**Deliverables:**
- Third-party audit engagement
- Audit findings remediation
- Penetration testing
- Cryptographic review

**Acceptance Criteria:**
- [ ] External audit completed
- [ ] All critical findings resolved
- [ ] All high findings resolved
- [ ] Audit report published

#### Sprint 4.3: Probing Resistance (Weeks 37-38)

**Deliverables:**
- Active probing tests
- Proof-of-knowledge validation
- Response timing analysis
- Fingerprint resistance verification

```rust
// Key milestone: Probing Resistance Test Suite
#[cfg(test)]
mod probing_tests {
    use super::*;

    #[test]
    fn test_invalid_probe_rejection() {
        let server = TestServer::new();

        // Random data should be silently dropped
        let response = server.send_and_wait(
            &random_bytes(100),
            Duration::from_secs(5),
        );
        assert!(response.is_none());
    }

    #[test]
    fn test_replay_rejection() {
        let server = TestServer::new();
        let valid_probe = create_valid_probe(&server.public_key());

        // First attempt should work
        let r1 = server.send_and_wait(&valid_probe, Duration::from_secs(5));
        assert!(r1.is_some());

        // Replay should be rejected
        let r2 = server.send_and_wait(&valid_probe, Duration::from_secs(5));
        assert!(r2.is_none());
    }

    #[test]
    fn test_timing_uniformity() {
        let server = TestServer::new();
        let mut timings = Vec::new();

        for _ in 0..1000 {
            let start = Instant::now();
            let _ = server.send_and_wait(
                &random_bytes(100),
                Duration::from_secs(1),
            );
            timings.push(start.elapsed());
        }

        // Verify timing is uniform (within statistical bounds)
        assert!(timing_is_uniform(&timings));
    }
}
```

**Acceptance Criteria:**
- [ ] No fingerprinting possible
- [ ] Invalid probes get no response
- [ ] Timing side channels eliminated
- [ ] All probing tests pass

#### Sprint 4.4: Timing Attack Mitigation (Weeks 39-40)

**Deliverables:**
- Constant-time implementations
- Timing noise injection
- Side-channel analysis
- Performance regression checks

**Acceptance Criteria:**
- [ ] All crypto ops constant-time
- [ ] Timing analysis shows no leakage
- [ ] Performance impact acceptable
- [ ] Side-channel tests pass

### Phase 4 Exit Criteria

- [ ] External audit complete
- [ ] All findings remediated
- [ ] Probing resistance verified
- [ ] Timing attacks mitigated

---

## Phase 5: Production Readiness

**Duration:** Weeks 37-44 (8 weeks, overlaps Phase 4)
**Team Size:** 6 FTE
**Story Points:** ~250

### Objectives

1. Complete documentation
2. Performance optimization
3. Release preparation
4. Deployment tooling

### Sprint Breakdown

#### Sprint 5.1: Documentation (Weeks 37-38)

**Deliverables:**
- API documentation (rustdoc)
- User guide
- Migration guide
- Security whitepaper

**Acceptance Criteria:**
- [ ] All public APIs documented
- [ ] User guide covers all features
- [ ] Migration guide tested
- [ ] Whitepaper published

#### Sprint 5.2: Performance Optimization (Weeks 39-40)

**Deliverables:**
- Profiling and optimization
- Memory usage reduction
- Latency optimization
- Throughput maximization

```rust
// Key milestone: Performance Benchmarks
pub mod benchmarks {
    use criterion::{criterion_group, Criterion};

    pub fn handshake_benchmark(c: &mut Criterion) {
        c.bench_function("hybrid_handshake", |b| {
            b.iter(|| {
                let (client, server) = create_test_pair();
                complete_handshake(client, server)
            })
        });
    }

    pub fn throughput_benchmark(c: &mut Criterion) {
        let mut group = c.benchmark_group("throughput");

        for size in [1024, 65536, 1048576].iter() {
            group.throughput(Throughput::Bytes(*size as u64));
            group.bench_with_input(
                BenchmarkId::from_parameter(size),
                size,
                |b, &size| {
                    b.iter(|| transfer_data(size))
                },
            );
        }

        group.finish();
    }
}
```

**Acceptance Criteria:**
- [ ] Handshake < 50ms
- [ ] Throughput > 300 Mbps (userspace)
- [ ] Throughput > 10 Gbps (kernel bypass)
- [ ] Memory < 100MB per 1000 sessions

#### Sprint 5.3: Release Preparation (Weeks 41-42)

**Deliverables:**
- Release candidate builds
- Integration testing
- Upgrade path validation
- Changelog finalization

**Acceptance Criteria:**
- [ ] RC builds on all platforms
- [ ] Integration tests pass
- [ ] Upgrade path verified
- [ ] Changelog complete

#### Sprint 5.4: Deployment & Launch (Weeks 43-44)

**Deliverables:**
- Production deployment guide
- Monitoring integration
- Alerting setup
- Launch announcement

**Acceptance Criteria:**
- [ ] Deployment guide tested
- [ ] Monitoring operational
- [ ] Alerts configured
- [ ] v2.0.0 released

### Phase 5 Exit Criteria

- [ ] Documentation complete
- [ ] Performance targets met
- [ ] v2.0.0 released
- [ ] Deployment tooling ready

---

## Resource Requirements

### Team Composition

| Role | FTE | Phases |
|------|-----|--------|
| Crypto Engineer | 2 | 1-4 |
| Systems Programmer | 4 | 1-5 |
| Security Engineer | 2 | 3-4 |
| QA Engineer | 2 | 2-5 |
| Technical Writer | 1 | 4-5 |
| DevOps Engineer | 1 | 3-5 |
| **Total** | **12** | |

### Infrastructure

| Resource | Specification | Purpose |
|----------|---------------|---------|
| Build Servers | 4x 32-core, 128GB RAM | CI/CD |
| Test Cluster | 8x high-perf nodes | Integration testing |
| Fuzzing Farm | 16x dedicated cores | Security testing |
| Documentation | Static site hosting | Docs hosting |

### Budget Estimate

| Category | Estimate |
|----------|----------|
| Personnel (12 FTE x 11 months) | $1.8M - $2.4M |
| Infrastructure | $50K - $100K |
| External Audit | $100K - $200K |
| Tools & Licenses | $20K - $50K |
| Contingency (15%) | $300K - $400K |
| **Total** | **$2.3M - $3.2M** |

---

## Risk Assessment

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| PQ crypto perf issues | Medium | High | Early benchmarking, fallback mode |
| AF_XDP compatibility | Low | Medium | Graceful degradation |
| Protocol complexity | Medium | High | Extensive testing, formal methods |
| Platform support gaps | Medium | Medium | Prioritize Linux, phase other platforms |

### Schedule Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| External audit delays | Medium | High | Early engagement, parallel work |
| Crypto spec changes | Low | High | Monitor NIST, design for flexibility |
| Integration challenges | Medium | Medium | Early integration testing |
| Resource availability | Medium | Medium | Cross-training, documentation |

### Mitigation Strategies

1. **Parallel Development**: Multiple streams reduce critical path
2. **Early Integration**: Continuous integration catches issues early
3. **Feature Flags**: Allows partial releases and rollback
4. **Extensive Testing**: Reduces production issues

---

## Success Criteria

### Phase 1 Success Metrics

- [ ] Crypto primitives pass all NIST vectors
- [ ] Wire format v2 fully functional
- [ ] Compatibility layer handles v1 connections
- [ ] CI/CD achieving > 80% coverage

### Phase 2 Success Metrics

- [ ] Handshake completes in < 2 RTT
- [ ] Per-packet ratchet verified
- [ ] Polymorphic format undetectable
- [ ] 1000+ concurrent streams

### Phase 3 Success Metrics

- [ ] All transport types working
- [ ] AF_XDP: > 10 Gbps throughput
- [ ] Transport migration < 50ms
- [ ] Zero packet loss during migration

### Phase 4 Success Metrics

- [ ] External audit: no critical issues
- [ ] Fuzzing: > 10B iterations, no crashes
- [ ] Probing: 100% rejection rate
- [ ] Timing: < 1% variance

### Phase 5 Success Metrics

- [ ] Documentation: 100% API coverage
- [ ] Performance: all targets met
- [ ] Release: v2.0.0 published
- [ ] Deployment: 3+ production users

---

## Related Documents

- [Development Plan](04-WRAITH-Protocol-v2-Development-Plan.md) - Detailed timeline
- [Architecture](02-WRAITH-Protocol-v2-Architecture.md) - System design
- [Specification](01-WRAITH-Protocol-v2-Specification.md) - Technical spec
- [Testing Strategy](14-WRAITH-Protocol-v2-Testing-Strategy.md) - Test approach

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial implementation phases document |
