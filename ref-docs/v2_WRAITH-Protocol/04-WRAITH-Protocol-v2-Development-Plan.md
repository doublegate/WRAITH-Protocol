# WRAITH Protocol v2 Development and Migration Plan

**Document Version:** 2.0.0  
**Status:** Implementation Roadmap  
**Date:** January 2026  

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Development Philosophy](#2-development-philosophy)
3. [Phase Overview](#3-phase-overview)
4. [Phase 1: Foundation](#4-phase-1-foundation)
5. [Phase 2: Core Protocol](#5-phase-2-core-protocol)
6. [Phase 3: Advanced Features](#6-phase-3-advanced-features)
7. [Phase 4: Platform Expansion](#7-phase-4-platform-expansion)
8. [Phase 5: Production Hardening](#8-phase-5-production-hardening)
9. [Migration Strategy](#9-migration-strategy)
10. [Testing Strategy](#10-testing-strategy)
11. [Risk Management](#11-risk-management)
12. [Resource Requirements](#12-resource-requirements)
13. [Milestones and Timeline](#13-milestones-and-timeline)

---

## 1. Executive Summary

### 1.1 Purpose

This document outlines the development plan for implementing WRAITH Protocol v2 from the v1 codebase, including technical milestones, migration strategies, and resource requirements.

### 1.2 Scope

The development plan covers:
- Incremental migration from v1 to v2
- New feature implementation
- Cross-platform expansion
- Testing and validation
- Production deployment

### 1.3 Timeline Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        WRAITH v2 Development Timeline                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Q1 2026          Q2 2026          Q3 2026          Q4 2026                │
│  ──────────────────────────────────────────────────────────────────────    │
│                                                                             │
│  ├─ Phase 1 ─┤├─── Phase 2 ───┤├─── Phase 3 ───┤├─ Phase 4 ─┤├─ Ph 5 ─┤   │
│  Foundation    Core Protocol    Advanced Feat.   Platforms    Production   │
│                                                                             │
│  Key Milestones:                                                           │
│  • M1: Transport abstraction complete (Week 4)                             │
│  • M2: Hybrid crypto implemented (Week 10)                                 │
│  • M3: Polymorphic wire format (Week 14)                                   │
│  • M4: Group support alpha (Week 22)                                       │
│  • M5: Cross-platform beta (Week 32)                                       │
│  • M6: Production release (Week 42)                                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Development Philosophy

### 2.1 Core Principles

1. **Incremental Migration**: Each phase produces working software
2. **Backward Compatibility Layer**: Temporary v1 compatibility where possible
3. **Test-Driven Development**: Comprehensive tests before implementation
4. **Security-First**: Security review at each phase gate
5. **Performance Baselines**: Continuous benchmarking against v1

### 2.2 Development Approach

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Development Workflow                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                    ┌──────────────────┐                                    │
│                    │  Specification   │                                    │
│                    │    Document      │                                    │
│                    └────────┬─────────┘                                    │
│                             │                                              │
│                             ▼                                              │
│                    ┌──────────────────┐                                    │
│                    │  Test Vectors    │                                    │
│                    │  & Fuzzing       │                                    │
│                    └────────┬─────────┘                                    │
│                             │                                              │
│         ┌───────────────────┼───────────────────┐                          │
│         │                   │                   │                          │
│         ▼                   ▼                   ▼                          │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                    │
│  │    Unit     │    │Integration │    │  Property   │                    │
│  │   Tests     │    │   Tests    │    │   Tests     │                    │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘                    │
│         │                  │                  │                           │
│         └──────────────────┼──────────────────┘                           │
│                            │                                              │
│                            ▼                                              │
│                    ┌──────────────────┐                                    │
│                    │  Implementation  │                                    │
│                    └────────┬─────────┘                                    │
│                             │                                              │
│                             ▼                                              │
│                    ┌──────────────────┐                                    │
│                    │ Security Review  │                                    │
│                    └────────┬─────────┘                                    │
│                             │                                              │
│                             ▼                                              │
│                    ┌──────────────────┐                                    │
│                    │   Performance    │                                    │
│                    │   Benchmarks     │                                    │
│                    └────────┬─────────┘                                    │
│                             │                                              │
│                             ▼                                              │
│                    ┌──────────────────┐                                    │
│                    │     Merge        │                                    │
│                    └──────────────────┘                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.3 Branch Strategy

```
main (v1 stable)
│
├── v2-development (integration branch)
│   │
│   ├── feature/transport-abstraction
│   ├── feature/hybrid-crypto
│   ├── feature/polymorphic-wire
│   ├── feature/probing-resistance
│   ├── feature/group-support
│   ├── feature/realtime-qos
│   ├── feature/platform-windows
│   ├── feature/platform-macos
│   └── feature/platform-wasm
│
└── v2-release (stabilization branch)
```

---

## 3. Phase Overview

### 3.1 Phase Summary

| Phase | Name | Duration | Key Deliverables |
|-------|------|----------|------------------|
| 1 | Foundation | 6 weeks | Transport abstraction, platform traits, test infrastructure |
| 2 | Core Protocol | 10 weeks | Hybrid crypto, polymorphic wire, obfuscation engine |
| 3 | Advanced Features | 10 weeks | Groups, real-time QoS, content addressing |
| 4 | Platform Expansion | 8 weeks | Windows, macOS, WASM support |
| 5 | Production Hardening | 8 weeks | Security audit, performance optimization, documentation |

### 3.2 Dependencies

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Phase Dependencies                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Phase 1: Foundation                                                       │
│  └── No dependencies (greenfield infrastructure)                           │
│                                                                             │
│  Phase 2: Core Protocol                                                    │
│  ├── Depends on: Phase 1 (transport abstraction)                          │
│  └── Parallel work: Crypto and wire format can proceed simultaneously     │
│                                                                             │
│  Phase 3: Advanced Features                                                │
│  ├── Depends on: Phase 2 (core protocol complete)                         │
│  ├── Groups depend on: Session management                                 │
│  ├── Real-time depends on: Stream multiplexing                            │
│  └── Content addressing: Independent, can start in Phase 2                │
│                                                                             │
│  Phase 4: Platform Expansion                                               │
│  ├── Depends on: Phase 1 (platform abstraction)                           │
│  └── Can proceed in parallel with Phase 3                                 │
│                                                                             │
│  Phase 5: Production Hardening                                             │
│  └── Depends on: All previous phases feature-complete                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Phase 1: Foundation

### 4.1 Objectives

- Establish platform abstraction layer
- Implement transport abstraction
- Set up comprehensive test infrastructure
- Create v1 compatibility shim

### 4.2 Work Items

#### 4.2.1 Platform Abstraction (Weeks 1-2)

```rust
// Target interface
pub trait Platform: Send + Sync + 'static {
    type Socket: AsyncRead + AsyncWrite + Send;
    type Timer: Future<Output = ()> + Send;
    type Random: CryptoRng + Send;
    
    fn create_udp_socket(&self, addr: SocketAddr) -> Result<Self::Socket>;
    fn create_tcp_socket(&self, addr: SocketAddr) -> Result<Self::Socket>;
    fn sleep(&self, duration: Duration) -> Self::Timer;
    fn random(&self) -> Self::Random;
    fn now(&self) -> Instant;
    fn features(&self) -> FeatureFlags;
}

// Implementation targets
// - NativePlatform (Linux) - Week 1
// - Platform trait definition - Week 1
// - FeatureFlags enumeration - Week 2
// - Platform detection logic - Week 2
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Define Platform trait | 2 days | Core | ☐ |
| Implement LinuxPlatform | 3 days | Core | ☐ |
| Feature flag system | 2 days | Core | ☐ |
| Platform detection | 1 day | Core | ☐ |
| Unit tests | 2 days | Test | ☐ |

#### 4.2.2 Transport Abstraction (Weeks 2-4)

```rust
// Target interface
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, packet: &[u8]) -> Result<(), TransportError>;
    async fn recv(&self, buf: &mut [u8]) -> Result<usize, TransportError>;
    fn characteristics(&self) -> TransportCharacteristics;
    fn mtu(&self) -> usize;
    fn local_addr(&self) -> Result<SocketAddr, TransportError>;
    fn peer_addr(&self) -> Option<SocketAddr>;
    async fn close(&self) -> Result<(), TransportError>;
}

// Implementation priority
// 1. UDP Transport - Week 2 (v1 compatible)
// 2. TCP Transport - Week 3
// 3. WebSocket Transport - Week 3-4
// 4. Transport selection logic - Week 4
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Define Transport trait | 1 day | Core | ☐ |
| UDP implementation | 3 days | Core | ☐ |
| TCP implementation | 3 days | Core | ☐ |
| WebSocket implementation | 4 days | Core | ☐ |
| Transport characteristics | 2 days | Core | ☐ |
| Selection logic | 2 days | Core | ☐ |
| Integration tests | 3 days | Test | ☐ |

#### 4.2.3 Test Infrastructure (Weeks 3-5)

```rust
// Test framework components
pub mod test_infra {
    // Network simulation
    pub struct NetworkSimulator {
        latency: Distribution,
        loss_rate: f64,
        bandwidth: Bandwidth,
        jitter: Duration,
    }
    
    // Packet capture and analysis
    pub struct PacketCapture {
        packets: Vec<CapturedPacket>,
    }
    
    // Fuzzing harness
    pub struct ProtocolFuzzer {
        corpus: Vec<Vec<u8>>,
        coverage: CoverageMap,
    }
    
    // Interoperability test runner
    pub struct InteropTestRunner {
        v1_binary: PathBuf,
        v2_binary: PathBuf,
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Network simulator | 4 days | Test | ☐ |
| Packet capture framework | 2 days | Test | ☐ |
| Fuzzing harness (libfuzzer) | 3 days | Test | ☐ |
| Property-based test framework | 2 days | Test | ☐ |
| CI/CD pipeline updates | 2 days | DevOps | ☐ |
| Benchmark infrastructure | 3 days | Test | ☐ |

#### 4.2.4 v1 Compatibility Layer (Weeks 5-6)

```rust
// Compatibility shim for gradual migration
pub mod v1_compat {
    /// Wrap v2 session to behave like v1
    pub struct V1CompatSession {
        inner: v2::Session,
    }
    
    impl V1CompatSession {
        /// Create with v1-like API
        pub fn connect(addr: SocketAddr, key: &StaticKey) -> Result<Self> {
            let config = SessionConfig::v1_compatible();
            let inner = v2::Session::connect(addr, key, config)?;
            Ok(Self { inner })
        }
    }
    
    /// v1-compatible configuration
    impl SessionConfig {
        pub fn v1_compatible() -> Self {
            Self {
                transport: TransportType::Udp,
                post_quantum: false,
                wire_format: WireFormat::V1Fixed,
                padding: PaddingConfig::v1_classes(),
                // ... other v1 defaults
            }
        }
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| V1 API compatibility shim | 3 days | Core | ☐ |
| V1 wire format support | 2 days | Core | ☐ |
| V1 configuration mapping | 1 day | Core | ☐ |
| Interop tests v1↔v1-compat | 2 days | Test | ☐ |

### 4.3 Phase 1 Exit Criteria

- [ ] All transports (UDP, TCP, WebSocket) passing unit tests
- [ ] Platform abstraction working on Linux
- [ ] Test infrastructure operational
- [ ] v1 compatibility layer passing interop tests
- [ ] Performance baseline established (v1 benchmarks)
- [ ] Code review complete
- [ ] Documentation updated

### 4.4 Phase 1 Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Transport abstraction too slow | High | Medium | Profile early, optimize hot paths |
| v1 compat breaks v1 deployments | High | Low | Extensive interop testing |
| Test infrastructure delays | Medium | Medium | Parallel development track |

---

## 5. Phase 2: Core Protocol

### 5.1 Objectives

- Implement hybrid post-quantum cryptography
- Create polymorphic wire format
- Build enhanced obfuscation engine
- Complete probing resistance

### 5.2 Work Items

#### 5.2.1 Hybrid Cryptography (Weeks 7-10)

```rust
// Implementation targets
pub mod crypto_v2 {
    // ML-KEM-768 integration
    pub struct MlKem768 {
        // NIST FIPS 203 implementation
    }
    
    // Hybrid key exchange
    pub struct HybridKem {
        classical: X25519,
        post_quantum: MlKem768,
    }
    
    impl HybridKem {
        pub fn encapsulate(&self, peer_pk: &HybridPublicKey) 
            -> (SharedSecret, HybridCiphertext) 
        {
            let (c_ss, c_ct) = self.classical.encapsulate(&peer_pk.classical);
            let (pq_ss, pq_ct) = self.post_quantum.encapsulate(&peer_pk.post_quantum);
            
            let combined = blake3::derive_key(
                "wraith-hybrid-kem-v2",
                &[c_ss.as_bytes(), pq_ss.as_bytes()].concat()
            );
            
            (SharedSecret(combined), HybridCiphertext { c_ct, pq_ct })
        }
    }
    
    // Extended Noise_XX handshake
    pub struct NoiseXXHybrid {
        // Noise framework with hybrid KEM extension
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| ML-KEM-768 integration (pqcrypto crate) | 3 days | Crypto | ☐ |
| Hybrid KEM combiner | 2 days | Crypto | ☐ |
| Noise_XX extension for hybrid | 5 days | Crypto | ☐ |
| Key derivation updates | 2 days | Crypto | ☐ |
| Ratcheting with PQ refresh | 3 days | Crypto | ☐ |
| Cryptographic agility framework | 3 days | Crypto | ☐ |
| Test vectors (NIST KAT) | 2 days | Test | ☐ |
| Security review checkpoint | 3 days | Security | ☐ |

#### 5.2.2 Polymorphic Wire Format (Weeks 9-12)

```rust
// Implementation targets
pub mod wire_v2 {
    // Wire format specification
    pub struct WireFormatSpec {
        fields: Vec<WireField>,
        fixed_overhead: usize,
    }
    
    // Format derivation from session secret
    impl WireFormatSpec {
        pub fn derive(session_secret: &[u8; 32]) -> Self {
            let seed = blake3::derive_key("wire-format-v2", session_secret);
            let mut rng = ChaCha20Rng::from_seed(seed);
            
            // Randomize field order and sizes
            Self::generate_random_format(&mut rng)
        }
    }
    
    // Encoder/decoder
    pub struct WireCodec {
        spec: WireFormatSpec,
    }
    
    impl WireCodec {
        pub fn encode(&self, frame: &Frame) -> Vec<u8>;
        pub fn decode(&self, packet: &[u8]) -> Result<Frame, DecodeError>;
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Wire format specification | 2 days | Core | ☐ |
| Format derivation algorithm | 3 days | Core | ☐ |
| Wire encoder | 4 days | Core | ☐ |
| Wire decoder | 4 days | Core | ☐ |
| V1 format compatibility mode | 2 days | Core | ☐ |
| Fuzzing (malformed packets) | 3 days | Test | ☐ |
| Performance benchmarks | 2 days | Test | ☐ |

#### 5.2.3 Enhanced Obfuscation (Weeks 11-14)

```rust
// Implementation targets
pub mod obfuscation_v2 {
    // Elligator2 encoding
    pub fn elligator2_encode(pk: &PublicKey) -> Option<Representative>;
    pub fn elligator2_decode(repr: &Representative) -> PublicKey;
    
    // Continuous padding distribution
    pub struct PaddingEngine {
        distribution: PaddingDistribution,
        rng: ChaCha20Rng,
    }
    
    // Timing obfuscation
    pub struct TimingObfuscator {
        mode: TimingMode,
        state: TimingState,
    }
    
    // Probing resistance
    pub struct ProbingResistance {
        require_proof: bool,
        probe_response: ProbeResponse,
    }
    
    // Entropy normalization
    pub struct EntropyNormalizer {
        method: EntropyMethod,
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Elligator2 implementation | 3 days | Crypto | ☐ |
| Continuous padding distributions | 4 days | Core | ☐ |
| HTTPS empirical distribution | 2 days | Core | ☐ |
| Timing obfuscator framework | 5 days | Core | ☐ |
| HMM timing models | 4 days | Core | ☐ |
| Proof-of-knowledge system | 3 days | Core | ☐ |
| Protocol mimicry (TLS response) | 4 days | Core | ☐ |
| Service fronting support | 3 days | Core | ☐ |
| Entropy normalization | 3 days | Core | ☐ |
| Traffic analysis tests | 4 days | Test | ☐ |

#### 5.2.4 Session Management Updates (Weeks 13-16)

```rust
// Implementation targets
pub mod session_v2 {
    // Extended session states
    pub enum SessionState {
        Closed,
        Connecting,
        Established,
        Rekeying,
        Migrating,
        Resuming,
        Draining,
    }
    
    // Session resumption
    pub struct ResumptionTicket {
        id: [u8; 16],
        resumption_secret: [u8; 32],
        expires: SystemTime,
        encrypted_params: Vec<u8>,
    }
    
    // Extension negotiation
    pub struct ExtensionNegotiator {
        offered: Vec<Extension>,
        accepted: Vec<Extension>,
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Session state machine updates | 3 days | Core | ☐ |
| Session resumption | 4 days | Core | ☐ |
| Extension negotiation | 3 days | Core | ☐ |
| Connection migration updates | 3 days | Core | ☐ |
| CID rotation | 2 days | Core | ☐ |
| Integration tests | 4 days | Test | ☐ |

### 5.3 Phase 2 Exit Criteria

- [ ] Hybrid key exchange passing NIST test vectors
- [ ] Polymorphic wire format operational
- [ ] Obfuscation engine complete
- [ ] Probing resistance tested against active probes
- [ ] Session resumption working
- [ ] All v2 features togglable for v1 compat mode
- [ ] Security review checkpoint passed
- [ ] Performance within 10% of v1 baseline

### 5.4 Phase 2 Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| ML-KEM integration issues | High | Low | Use well-tested pqcrypto crate |
| Wire format parsing bugs | High | Medium | Extensive fuzzing |
| Obfuscation adds latency | Medium | Medium | Optional features, profiling |
| Handshake size increase (PQ) | Low | High | Document trade-offs |

---

## 6. Phase 3: Advanced Features

### 6.1 Objectives

- Implement group communication with TreeKEM
- Add real-time QoS and FEC support
- Create content-addressed storage layer
- Build resource profile system

### 6.2 Work Items

#### 6.2.1 Group Communication (Weeks 17-22)

```rust
// Implementation targets
pub mod groups {
    // Group session
    pub struct GroupSession {
        group_id: GroupId,
        members: HashMap<PeerId, MemberInfo>,
        topology: GroupTopology,
        tree_kem: TreeKem,
    }
    
    // TreeKEM implementation
    pub struct TreeKem {
        nodes: Vec<TreeNode>,
        position: usize,
        leaf_secret: Zeroizing<[u8; 32]>,
    }
    
    impl TreeKem {
        pub fn self_update(&mut self) -> KeyUpdate;
        pub fn process_update(&mut self, update: &KeyUpdate) -> Result<()>;
        pub fn add_member(&mut self, pk: &PublicKey) -> Result<KeyUpdate>;
        pub fn remove_member(&mut self, position: usize) -> Result<KeyUpdate>;
    }
    
    // Group frame types
    pub enum GroupFrame {
        Join(GroupJoinFrame),
        Leave(GroupLeaveFrame),
        Rekey(GroupRekeyFrame),
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| TreeKEM core implementation | 8 days | Crypto | ☐ |
| TreeKEM add/remove operations | 5 days | Crypto | ☐ |
| Group session management | 5 days | Core | ☐ |
| Group frame types | 3 days | Core | ☐ |
| Topology modes (mesh, tree, gossip) | 6 days | Core | ☐ |
| Group discovery (DHT integration) | 4 days | Core | ☐ |
| Multi-party NAT traversal | 4 days | Core | ☐ |
| Integration tests | 5 days | Test | ☐ |
| Security review | 3 days | Security | ☐ |

#### 6.2.2 Real-Time QoS (Weeks 19-24)

```rust
// Implementation targets
pub mod realtime {
    // QoS modes
    pub enum QosMode {
        Reliable,
        UnreliableOrdered { max_age: Duration },
        UnreliableUnordered,
        PartiallyReliable { max_retransmits: u8 },
    }
    
    // FEC encoder/decoder
    pub struct FecCodec {
        algorithm: FecAlgorithm,
        redundancy: f32,
        block_size: usize,
    }
    
    impl FecCodec {
        pub fn encode(&self, packets: &[Packet]) -> Vec<Packet>;
        pub fn decode(&self, packets: &[Packet]) -> Result<Vec<Packet>>;
    }
    
    // Jitter buffer
    pub struct JitterBuffer {
        target_latency: Duration,
        buffer: BinaryHeap<TimedPacket>,
    }
    
    // Priority scheduler
    pub struct PriorityScheduler {
        queues: [VecDeque<Packet>; 8],  // 8 priority levels
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| QoS mode framework | 3 days | Core | ☐ |
| Unreliable delivery modes | 4 days | Core | ☐ |
| FEC: XOR codec | 2 days | Core | ☐ |
| FEC: Reed-Solomon codec | 4 days | Core | ☐ |
| Jitter buffer | 3 days | Core | ☐ |
| Priority scheduler | 3 days | Core | ☐ |
| Stream priority frames | 2 days | Core | ☐ |
| Datagram frames | 2 days | Core | ☐ |
| Real-time benchmarks | 3 days | Test | ☐ |

#### 6.2.3 Content-Addressed Storage (Weeks 21-26)

```rust
// Implementation targets
pub mod content_addressed {
    // Merkle tree file representation
    pub struct MerkleFile {
        root: Hash,
        size: u64,
        tree: MerkleTree,
    }
    
    // Content-defined chunking
    pub struct Chunker {
        algorithm: ChunkingAlgorithm,
    }
    
    pub enum ChunkingAlgorithm {
        Fixed { size: usize },
        ContentDefined { 
            min: usize, 
            max: usize, 
            avg: usize 
        },
    }
    
    // Resumable transfer state
    pub struct ResumableTransfer {
        file: MerkleFile,
        received_chunks: BitVec,
        pending_chunks: HashSet<ChunkIndex>,
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Merkle tree implementation | 4 days | Core | ☐ |
| Fixed chunking | 2 days | Core | ☐ |
| Content-defined chunking (Rabin) | 4 days | Core | ☐ |
| Chunk verification | 2 days | Core | ☐ |
| Resumable transfer state | 3 days | Core | ☐ |
| Delta sync (diff) | 4 days | Core | ☐ |
| Deduplication | 3 days | Core | ☐ |
| Integration tests | 3 days | Test | ☐ |

#### 6.2.4 Resource Profiles (Weeks 23-26)

```rust
// Implementation targets
pub mod profiles {
    pub enum ResourceProfile {
        Performance { 
            kernel_bypass: bool,
            memory_budget: ByteSize,
        },
        Balanced { 
            max_cpu_percent: f32,
            max_memory: ByteSize,
        },
        Constrained { 
            power_mode: PowerMode,
            max_memory: ByteSize,
        },
        Stealth { 
            max_bandwidth: Bandwidth,
            shaping: TrafficProfile,
        },
        Metered { 
            data_budget: ByteSize,
            budget_period: Duration,
        },
    }
    
    impl ProtocolConfig {
        pub fn apply_profile(&mut self, profile: ResourceProfile);
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Profile framework | 2 days | Core | ☐ |
| Performance profile | 3 days | Core | ☐ |
| Constrained profile | 3 days | Core | ☐ |
| Stealth profile | 3 days | Core | ☐ |
| Metered profile | 2 days | Core | ☐ |
| Profile auto-detection | 2 days | Core | ☐ |
| Profile switching at runtime | 2 days | Core | ☐ |
| Profile benchmarks | 3 days | Test | ☐ |

### 6.3 Phase 3 Exit Criteria

- [ ] Group sessions working with 10+ members
- [ ] TreeKEM passing security review
- [ ] Real-time streaming achieving target latency
- [ ] FEC recovering from 20% packet loss
- [ ] Content addressing enabling deduplication
- [ ] All profiles functional
- [ ] Performance benchmarks documented

### 6.4 Phase 3 Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| TreeKEM complexity | High | Medium | Follow MLS spec closely |
| FEC latency overhead | Medium | Low | Make FEC optional |
| Group scalability | Medium | Medium | Document limits |
| CDC algorithm patents | Low | Low | Use Rabin fingerprinting |

---

## 7. Phase 4: Platform Expansion

### 7.1 Objectives

- Port to Windows
- Port to macOS
- Implement WASM/browser support
- Create embedded/no_std core

### 7.2 Work Items

#### 7.2.1 Windows Support (Weeks 27-30)

```rust
// Implementation targets
#[cfg(target_os = "windows")]
pub mod platform_windows {
    pub struct WindowsPlatform {
        // IOCP-based async I/O
    }
    
    impl Platform for WindowsPlatform {
        type Socket = WindowsSocket;
        type Timer = WindowsTimer;
        type Random = WindowsRng;
        
        // ... implementations
    }
    
    // Windows-specific transports
    pub struct WindowsUdpTransport { /* Winsock */ }
    pub struct WindowsTcpTransport { /* Winsock */ }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Windows platform trait impl | 4 days | Platform | ☐ |
| IOCP async wrapper | 3 days | Platform | ☐ |
| Winsock UDP transport | 3 days | Platform | ☐ |
| Winsock TCP transport | 2 days | Platform | ☐ |
| Windows crypto (CNG or ring) | 2 days | Platform | ☐ |
| VirtualLock for secure memory | 2 days | Platform | ☐ |
| Windows CI/CD | 2 days | DevOps | ☐ |
| Windows-specific tests | 3 days | Test | ☐ |

#### 7.2.2 macOS Support (Weeks 29-32)

```rust
// Implementation targets
#[cfg(target_os = "macos")]
pub mod platform_macos {
    pub struct MacOSPlatform {
        // kqueue-based async I/O
    }
    
    impl Platform for MacOSPlatform {
        type Socket = MacOSSocket;
        type Timer = MacOSTimer;
        type Random = MacOSRng;
        
        // ... implementations
    }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| macOS platform trait impl | 3 days | Platform | ☐ |
| kqueue async wrapper | 2 days | Platform | ☐ |
| BSD socket transports | 2 days | Platform | ☐ |
| macOS Keychain integration (optional) | 2 days | Platform | ☐ |
| macOS CI/CD | 2 days | DevOps | ☐ |
| macOS-specific tests | 2 days | Test | ☐ |

#### 7.2.3 WASM/Browser Support (Weeks 31-36)

```rust
// Implementation targets
#[cfg(target_arch = "wasm32")]
pub mod platform_wasm {
    pub struct WasmPlatform {
        // Browser APIs
    }
    
    impl Platform for WasmPlatform {
        type Socket = WebSocketWrapper;
        type Timer = JsTimer;
        type Random = WebCryptoRng;
        
        // ... implementations
    }
    
    // Browser-specific transports
    pub struct BrowserWebSocket { /* WebSocket API */ }
    pub struct BrowserWebRTC { /* RTCDataChannel */ }
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| WASM platform trait impl | 4 days | Platform | ☐ |
| WebSocket transport | 3 days | Platform | ☐ |
| WebRTC data channel transport | 5 days | Platform | ☐ |
| Web Crypto API integration | 3 days | Platform | ☐ |
| JavaScript bindings (wasm-bindgen) | 4 days | Platform | ☐ |
| npm package | 2 days | Platform | ☐ |
| Browser demo application | 3 days | Platform | ☐ |
| Browser-specific tests | 3 days | Test | ☐ |

#### 7.2.4 Embedded/no_std Core (Weeks 33-36)

```rust
// Implementation targets
#![no_std]
pub mod core_nostd {
    // Core protocol without std
    pub struct NoStdSession<T: Transport, A: Allocator> {
        // Allocator-aware implementation
    }
    
    // Feature flags for optional std components
    #[cfg(feature = "alloc")]
    extern crate alloc;
}
```

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Identify std dependencies | 2 days | Core | ☐ |
| Abstract allocator usage | 3 days | Core | ☐ |
| no_std crypto (ring or RustCrypto) | 3 days | Core | ☐ |
| no_std transport trait | 2 days | Core | ☐ |
| Feature flag system | 2 days | Core | ☐ |
| Embedded platform example | 2 days | Core | ☐ |
| Size optimization | 2 days | Core | ☐ |

### 7.3 Phase 4 Exit Criteria

- [ ] Windows build passing all tests
- [ ] macOS build passing all tests
- [ ] WASM demo working in Chrome, Firefox, Safari
- [ ] no_std core compiling for ARM Cortex-M
- [ ] Cross-platform CI green
- [ ] Platform-specific documentation

### 7.4 Phase 4 Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Windows async complexity | Medium | Medium | Use tokio-uring port |
| WASM crypto performance | Medium | Low | Use Web Crypto where possible |
| Browser API limitations | Medium | Medium | Document limitations |
| no_std allocation complexity | High | Medium | Optional alloc feature |

---

## 8. Phase 5: Production Hardening

### 8.1 Objectives

- Complete security audit
- Performance optimization
- Documentation completion
- Production deployment preparation

### 8.2 Work Items

#### 8.2.1 Security Audit (Weeks 37-40)

**Internal Review:**
| Area | Owner | Duration |
|------|-------|----------|
| Cryptographic implementation | Crypto Lead | 1 week |
| Protocol state machine | Core Lead | 1 week |
| Memory safety (unsafe audit) | Security | 1 week |
| Side-channel analysis | Security | 1 week |

**External Audit:**
| Audit Type | Vendor | Duration |
|------------|--------|----------|
| Cryptographic review | TBD | 2 weeks |
| Protocol analysis | TBD | 2 weeks |
| Code audit | TBD | 2 weeks |

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Prepare audit documentation | 3 days | Docs | ☐ |
| Internal crypto review | 5 days | Crypto | ☐ |
| Internal protocol review | 5 days | Core | ☐ |
| unsafe code audit | 3 days | Security | ☐ |
| External audit coordination | 2 days | PM | ☐ |
| Address audit findings | 10 days | All | ☐ |

#### 8.2.2 Performance Optimization (Weeks 39-42)

**Optimization Targets:**
| Metric | v1 Baseline | v2 Target | Stretch Goal |
|--------|-------------|-----------|--------------|
| Throughput (standard) | 1 Gbps | 2 Gbps | 5 Gbps |
| Throughput (bypass) | N/A | 40 Gbps | 100 Gbps |
| Latency (P50) | 5ms | 3ms | 1ms |
| Latency (P99) | 20ms | 10ms | 5ms |
| Memory per conn | 1 MB | 500 KB | 256 KB |
| Handshake time | 50ms | 40ms | 30ms |

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Profile hot paths | 3 days | Perf | ☐ |
| Optimize crypto paths | 4 days | Crypto | ☐ |
| Buffer pool implementation | 3 days | Core | ☐ |
| Lock-free data structures | 4 days | Core | ☐ |
| io_uring integration | 4 days | Core | ☐ |
| AF_XDP optimization | 5 days | Core | ☐ |
| Memory layout optimization | 3 days | Core | ☐ |
| Final benchmarks | 2 days | Test | ☐ |

#### 8.2.3 Documentation (Weeks 41-44)

**Documentation Deliverables:**
| Document | Audience | Status |
|----------|----------|--------|
| Technical Specification | Implementers | ☐ |
| Architecture Overview | Developers | ☐ |
| API Reference | Developers | ☐ |
| Security Analysis | Security teams | ☐ |
| Migration Guide | v1 users | ☐ |
| Deployment Guide | Operators | ☐ |
| Quick Start | New users | ☐ |

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Complete spec documentation | 5 days | Docs | ☐ |
| API documentation (rustdoc) | 3 days | Docs | ☐ |
| Example code | 3 days | Docs | ☐ |
| Tutorial creation | 4 days | Docs | ☐ |
| Deployment guide | 3 days | Docs | ☐ |
| Security whitepaper | 3 days | Security | ☐ |

#### 8.2.4 Release Preparation (Weeks 43-44)

**Tasks:**
| Task | Estimate | Owner | Status |
|------|----------|-------|--------|
| Release candidate build | 1 day | DevOps | ☐ |
| Release candidate testing | 3 days | Test | ☐ |
| Package for crates.io | 1 day | DevOps | ☐ |
| Package for npm (WASM) | 1 day | DevOps | ☐ |
| Release notes | 1 day | Docs | ☐ |
| Announcement preparation | 1 day | PM | ☐ |
| v1 deprecation notice | 1 day | PM | ☐ |

### 8.3 Phase 5 Exit Criteria

- [ ] Security audit passed with no critical findings
- [ ] Performance targets met
- [ ] All documentation complete
- [ ] Release candidate tested
- [ ] Packages ready for distribution
- [ ] Migration guide validated

---

## 9. Migration Strategy

### 9.1 Migration Approaches

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Migration Strategies                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Strategy A: Big Bang                                                      │
│  ─────────────────────                                                     │
│  • Upgrade all endpoints simultaneously                                    │
│  • Shortest migration window                                               │
│  • Highest risk                                                            │
│  • Best for: Small deployments, test environments                         │
│                                                                             │
│  Timeline: 1 day                                                           │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  Day 0: All v1 ──────────────► Day 1: All v2                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Strategy B: Rolling Upgrade                                               │
│  ───────────────────────────                                               │
│  • Upgrade endpoints in waves                                              │
│  • Gateway translates between versions                                     │
│  • Medium risk                                                             │
│  • Best for: Medium deployments                                           │
│                                                                             │
│  Timeline: 2-4 weeks                                                       │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  Week 1: Deploy v1↔v2 gateways                                       │   │
│  │  Week 2: Upgrade 25% clients to v2                                   │   │
│  │  Week 3: Upgrade remaining clients to v2                             │   │
│  │  Week 4: Remove gateways, decommission v1                           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Strategy C: Parallel Operation                                            │
│  ──────────────────────────────                                            │
│  • Run v1 and v2 networks simultaneously                                  │
│  • Clients choose version                                                  │
│  • Lowest risk                                                             │
│  • Best for: Large deployments, critical systems                          │
│                                                                             │
│  Timeline: 1-3 months                                                      │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  Month 1: Deploy v2 infrastructure alongside v1                      │   │
│  │  Month 2: Migrate willing users to v2                                │   │
│  │  Month 3: Deprecate v1, migrate remaining users                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 9.2 Gateway Architecture (Strategy B)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           v1↔v2 Gateway                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│            v1 Clients                          v2 Clients                  │
│               │                                    │                       │
│               ▼                                    ▼                       │
│        ┌─────────────┐                      ┌─────────────┐                │
│        │ v1 Protocol │                      │ v2 Protocol │                │
│        └──────┬──────┘                      └──────┬──────┘                │
│               │                                    │                       │
│               └────────────────┬───────────────────┘                       │
│                                │                                           │
│                        ┌───────▼───────┐                                   │
│                        │    Gateway    │                                   │
│                        │               │                                   │
│                        │  • Terminates │                                   │
│                        │    v1/v2 conn │                                   │
│                        │  • Translates │                                   │
│                        │    wire fmt   │                                   │
│                        │  • Handles    │                                   │
│                        │    crypto     │                                   │
│                        └───────┬───────┘                                   │
│                                │                                           │
│                        ┌───────▼───────┐                                   │
│                        │  v2 Backend   │                                   │
│                        └───────────────┘                                   │
│                                                                             │
│  Gateway Functions:                                                        │
│  • Accept v1 connections on port A                                        │
│  • Accept v2 connections on port B                                        │
│  • Translate v1 wire format to v2 internal                                │
│  • Handle v1 handshake, establish v2 session to backend                   │
│  • Forward traffic with format translation                                │
│                                                                             │
│  Limitations:                                                              │
│  • No post-quantum security for v1 clients                                │
│  • Limited obfuscation for v1 clients                                     │
│  • Additional latency (~1ms)                                              │
│  • Gateway becomes trust anchor                                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 9.3 Migration Checklist

```markdown
## Pre-Migration

- [ ] Inventory all v1 deployments
- [ ] Identify critical paths
- [ ] Test v2 in staging environment
- [ ] Train operations team on v2
- [ ] Prepare rollback procedure
- [ ] Schedule maintenance window
- [ ] Notify stakeholders

## Migration Day

- [ ] Deploy v2 infrastructure
- [ ] Verify v2 health checks
- [ ] Switch DNS/routing (if applicable)
- [ ] Monitor error rates
- [ ] Monitor latency
- [ ] Monitor throughput

## Post-Migration

- [ ] Verify all clients connected
- [ ] Run end-to-end tests
- [ ] Compare metrics to baseline
- [ ] Decommission v1 (after stability period)
- [ ] Update documentation
- [ ] Close migration project
```

---

## 10. Testing Strategy

### 10.1 Test Pyramid

```
                         ┌───────────────┐
                         │   End-to-End  │
                         │    Tests      │
                         │   (10 hours)  │
                         └───────┬───────┘
                                 │
                    ┌────────────┴────────────┐
                    │    Integration Tests    │
                    │       (2 hours)         │
                    └────────────┬────────────┘
                                 │
          ┌──────────────────────┴──────────────────────┐
          │              Unit Tests                      │
          │               (5 min)                        │
          └──────────────────────────────────────────────┘
```

### 10.2 Test Categories

| Category | Purpose | Count Target | Run Time |
|----------|---------|--------------|----------|
| Unit | Component correctness | 2000+ | <5 min |
| Integration | Component interaction | 500+ | <30 min |
| Property | Invariant verification | 200+ | <60 min |
| Fuzz | Edge case discovery | N/A | Continuous |
| Performance | Throughput/latency | 50+ | <30 min |
| Interop | v1/v2 compatibility | 100+ | <30 min |
| E2E | Full workflow | 100+ | <10 hours |

### 10.3 Test Vectors

```rust
// Cryptographic test vectors
pub mod test_vectors {
    // NIST ML-KEM test vectors
    pub const ML_KEM_768_ENCAPS: &[KemTestVector] = &[
        KemTestVector { pk: "...", ct: "...", ss: "..." },
        // ...
    ];
    
    // Noise_XX test vectors
    pub const NOISE_XX_HANDSHAKE: &[NoiseTestVector] = &[
        NoiseTestVector { 
            initiator_static: "...",
            responder_static: "...",
            messages: &["...", "...", "..."],
            session_keys: ("...", "..."),
        },
        // ...
    ];
    
    // Wire format test vectors
    pub const WIRE_FORMAT_V2: &[WireTestVector] = &[
        WireTestVector {
            session_secret: "...",
            expected_format: WireFormatSpec { /* ... */ },
        },
        // ...
    ];
}
```

### 10.4 Fuzzing Targets

| Target | Corpus | Priority |
|--------|--------|----------|
| Wire format decoder | Malformed packets | Critical |
| Handshake parser | Invalid handshakes | Critical |
| Frame parser | Corrupted frames | High |
| Crypto primitives | Random inputs | High |
| State machine | Random events | Medium |
| Configuration parser | Invalid config | Low |

---

## 11. Risk Management

### 11.1 Risk Register

| ID | Risk | Impact | Probability | Mitigation | Owner |
|----|------|--------|-------------|------------|-------|
| R1 | ML-KEM standardization changes | High | Low | Follow FIPS 203 final, modular design | Crypto |
| R2 | Performance regression | High | Medium | Continuous benchmarking | Perf |
| R3 | Security vulnerability | Critical | Low | Audit, fuzzing, responsible disclosure | Security |
| R4 | Schedule slip | Medium | Medium | Buffer time, prioritization | PM |
| R5 | Key personnel loss | High | Low | Documentation, knowledge sharing | PM |
| R6 | Platform-specific bugs | Medium | Medium | CI on all platforms | Test |
| R7 | Protocol ossification | Medium | Low | Extension framework | Core |
| R8 | Adoption resistance | Medium | Medium | Migration tools, documentation | PM |

### 11.2 Contingency Plans

**If ML-KEM changes:**
- Abstraction layer allows algorithm swap
- Fall back to classical-only mode
- Timeline: 2-4 weeks to adapt

**If performance targets not met:**
- Focus on critical paths only
- Defer kernel bypass to future release
- Timeline: 4 weeks to reassess

**If security issue found:**
- Responsible disclosure process
- Patch within 72 hours (critical)
- Timeline: Depends on severity

---

## 12. Resource Requirements

### 12.1 Team Structure

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Project Team                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                          Project Manager                                    │
│                               (1)                                           │
│                                │                                           │
│         ┌──────────────────────┼──────────────────────┐                    │
│         │                      │                      │                    │
│         ▼                      ▼                      ▼                    │
│  ┌─────────────┐        ┌─────────────┐        ┌─────────────┐            │
│  │  Core Team  │        │Crypto Team  │        │Platform Team│            │
│  │    (3)      │        │    (2)      │        │    (2)      │            │
│  │             │        │             │        │             │            │
│  │• Session    │        │• Crypto eng │        │• Windows    │            │
│  │  management │        │• Protocol   │        │• macOS      │            │
│  │• Transport  │        │  security   │        │• WASM       │            │
│  │• Wire format│        │             │        │             │            │
│  └─────────────┘        └─────────────┘        └─────────────┘            │
│                                                                             │
│         ┌──────────────────────┼──────────────────────┐                    │
│         │                      │                      │                    │
│         ▼                      ▼                      ▼                    │
│  ┌─────────────┐        ┌─────────────┐        ┌─────────────┐            │
│  │  Test Team  │        │Security Team│        │  DevOps     │            │
│  │    (2)      │        │    (1)      │        │    (1)      │            │
│  │             │        │             │        │             │            │
│  │• Testing    │        │• Security   │        │• CI/CD      │            │
│  │• Fuzzing    │        │  review     │        │• Release    │            │
│  │• Benchmarks │        │• Audit      │        │  engineering│            │
│  └─────────────┘        └─────────────┘        └─────────────┘            │
│                                                                             │
│  Total: 12 FTE                                                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 12.2 Infrastructure

| Resource | Purpose | Monthly Cost |
|----------|---------|--------------|
| CI/CD servers | Multi-platform builds | $500 |
| Test infrastructure | Network simulation | $300 |
| Cloud instances | Integration testing | $400 |
| Fuzzing cluster | Continuous fuzzing | $200 |
| External audit | Security review | $30,000 (one-time) |

### 12.3 Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| pqcrypto | 0.17+ | ML-KEM-768 |
| ring | 0.17+ | Classical crypto |
| tokio | 1.32+ | Async runtime |
| quinn | 0.10+ | QUIC transport |
| tungstenite | 0.21+ | WebSocket |
| wasm-bindgen | 0.2+ | WASM bindings |

---

## 13. Milestones and Timeline

### 13.1 Milestone Summary

| Milestone | Target Date | Deliverables |
|-----------|-------------|--------------|
| M1: Foundation Complete | Week 6 | Transport abstraction, platform traits |
| M2: Crypto Complete | Week 12 | Hybrid crypto, Noise_XX extended |
| M3: Core Protocol Complete | Week 16 | Polymorphic wire, obfuscation |
| M4: Advanced Features Alpha | Week 26 | Groups, real-time, content addressing |
| M5: Cross-Platform Beta | Week 36 | Windows, macOS, WASM |
| M6: Release Candidate | Week 42 | Security audit passed |
| M7: Production Release | Week 44 | v2.0.0 |

### 13.2 Detailed Timeline

```
Week   Phase 1          Phase 2          Phase 3          Phase 4          Phase 5
────────────────────────────────────────────────────────────────────────────────────
1-2    Platform abs.
3-4    Transport abs.
5-6    Test infra
       v1 compat
              ──────────M1──────────
7-8                     ML-KEM int.
9-10                    Hybrid KEM
                        Noise extend
11-12                   Wire format
              ──────────M2──────────
13-14                   Obfuscation
                        Probing res.
15-16                   Session mgmt
              ──────────M3──────────
17-18                                    TreeKEM
19-20                                    Group session
21-22                                    Real-time QoS
23-24                                    FEC
25-26                                    Content addr.
                                         Profiles
              ──────────M4──────────
27-28                                                     Windows
29-30                                                     Windows
                                                          macOS
31-32                                                     macOS
                                                          WASM
33-34                                                     WASM
35-36                                                     no_std
              ──────────M5──────────
37-38                                                                      Security audit
39-40                                                                      Perf optimize
41-42                                                                      Documentation
              ──────────M6──────────
43-44                                                                      Release prep
              ──────────M7: v2.0.0 Release──────────
```

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2026-01 | Initial development plan |

---

*End of WRAITH Protocol v2 Development and Migration Plan*
