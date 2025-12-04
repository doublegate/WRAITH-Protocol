# WRAITH Protocol

**W**ire-speed **R**esilient **A**uthenticated **I**nvisible **T**ransfer **H**andler

A decentralized secure file transfer protocol optimized for high-throughput, low-latency operation with strong security guarantees and traffic analysis resistance.

![WRAITH Protocol Banner](images/wraith-protocol_banner-graphic.jpg)

[![CI Status](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/ci.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/ci.yml)
[![CodeQL](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/codeql.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/codeql.yml)
[![Release](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/release.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/release.yml)
[![Version](https://img.shields.io/badge/version-0.9.0-blue.svg)](https://github.com/doublegate/WRAITH-Protocol/releases)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Edition](https://img.shields.io/badge/edition-2024-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

## Current Status

**Version:** 0.9.0 Beta (Node API Release) | **Latest Release**

WRAITH Protocol introduces the high-level Node API, providing a unified orchestration layer for protocol operations. The Node API integrates cryptography, transport, session management, and file transfer into a single cohesive interface.

**Phase 9 Complete (2025-12-03):**
- Node API & Protocol Orchestration (85 SP) - COMPLETE
  - Sprint 9.1: Node struct with lifecycle, session management, file transfer (34 SP)
  - Sprint 9.2: DHT integration, NAT traversal, connection lifecycle (21 SP)
  - Sprint 9.3: Traffic obfuscation integration (13 SP)
  - Sprint 9.4: Multi-peer downloads, integration tests, benchmarks (17 SP)
  - ~4,000 lines of new code across 9 modules
  - 57 comprehensive unit tests

**Progress: 887/947 story points delivered (94% overall)**

**Code Quality Metrics:**
- **Quality Grade:** A+ (95/100)
- **Technical Debt Ratio:** 12% (healthy range)
- **Test Coverage:** 791+ tests passing (722 library + 40 integration + 29 property) - 100% pass rate
  - 263 wraith-core (frame parsing, sessions, streams, BBR, migration, **Node API** with 57 new tests)
  - 125 wraith-crypto (Ed25519, X25519, Elligator2, AEAD, Noise, Ratchet, encryption at rest)
  - 24 wraith-files (chunking, reassembly, tree hashing, O(m) algorithms)
  - 154 wraith-obfuscation (padding, timing, TLS/WebSocket/DoH mimicry)
  - 15 wraith-discovery (DHT, STUN, ICE, relay)
  - 33 wraith-transport (AF_XDP, io_uring, UDP, worker pools)
  - 40 integration tests (end-to-end, Node API integration, cryptographic vectors)
  - 29 property tests (proptest invariants for state machines)
  - 108 doc tests (API examples across all crates)
- **Security Vulnerabilities:** Zero (cargo audit clean, CodeQL verified)
- **Clippy Warnings:** Zero
- **Code Volume:** ~36,600 lines of Rust code (~28,700 LOC + ~7,900 comments) across 7 active crates
- **Fuzzing:** 5 libFuzzer targets continuously testing parser robustness
  - frame_parser: SIMD/scalar frame parsing with arbitrary bytes
  - dht_message: Kademlia message handling (FIND_NODE, FIND_VALUE, STORE)
  - padding: All padding modes with round-trip validation
  - crypto: AEAD encrypt/decrypt and key derivation
  - tree_hash: Merkle tree construction with incremental hashing
- **Property Tests:** 29 proptest invariants validating state machine correctness
- **Unsafe Code:** 50 blocks with 100% SAFETY documentation (zero unsafe in crypto paths)
- **Documentation:** 60+ files, 45,000+ lines, complete API coverage, deployment guides

**Implementation Status:**
- **Core workspace:** 9 crates (8 active + 1 XDP), ~32,600 lines of Rust code (~24,700 LOC + ~7,900 comments)
- **Test coverage:** 980 total tests (969 active, 11 ignored) with 100% pass rate
  - **wraith-core** (219 tests): **Node API orchestration layer**, Frame parsing with SIMD acceleration (172M frames/sec), session state machine with 7 states, stream multiplexing with prioritization, BBR congestion control with pacing, path MTU discovery with caching, connection migration with PATH_CHALLENGE/RESPONSE, transfer session management
  - **wraith-crypto** (125 tests): Ed25519 signatures with batch verification, X25519 key exchange with Elligator2 encoding, XChaCha20-Poly1305 AEAD with key commitment (3.2 GB/s), BLAKE3 hashing with SIMD (8.5 GB/s), Noise_XX handshake with mutual authentication, Double Ratchet with DH and symmetric ratcheting, replay protection with 64-bit sliding window, private key encryption at rest (Argon2id + XChaCha20-Poly1305)
  - **wraith-files** (24 tests): io_uring async file I/O with registered buffers and zero-copy, file chunking with seek support (>1.5 GiB/s), file reassembly with O(m) missing chunks algorithm, BLAKE3 tree hashing with Merkle verification (>3 GiB/s), incremental tree hasher for streaming
  - **wraith-obfuscation** (154 tests): Padding engine with 5 modes (PowerOfTwo, SizeClasses, ConstantRate, Statistical), timing obfuscation with 5 distributions (Uniform, Normal, Exponential), TLS 1.3 record layer mimicry, WebSocket binary framing (RFC 6455), DNS-over-HTTPS tunneling, adaptive threat-level profiles (Low/Medium/High/Paranoid)
  - **wraith-discovery** (15 tests): Privacy-enhanced Kademlia DHT with BLAKE3 NodeIds, S/Kademlia Sybil resistance (20-bit difficulty), DHT privacy with keyed info_hash, STUN client (RFC 5389) with MESSAGE-INTEGRITY, ICE candidate gathering with UDP hole punching, DERP-style relay infrastructure (client/server/selector)
  - **wraith-transport** (33 tests): AF_XDP zero-copy sockets with batch processing (rx_batch/tx_batch), worker thread pools with CPU pinning, UDP transport with SO_REUSEPORT, MTU discovery with binary search, NUMA-aware allocation
  - **Integration & Benchmarks** (113 tests): End-to-end file transfer (5MB with resume), multi-peer coordination (3 peers, 20 chunks), NAT traversal components, relay fallback, obfuscation modes integration, Noise_XX + ratcheting workflow, cryptographic test vectors
  - **Doc tests** (303 tests): API documentation examples with runnable code across all crates
- **Benchmarks:** 28 Criterion benchmarks measuring frame parsing/building (~232 GiB/s theoretical), transport throughput/latency, MTU cache performance, worker pool scaling, obfuscation operation overhead, file chunking/reassembly, tree hashing throughput
- **Performance highlights:**
  - Frame parsing: 172M frames/sec with SIMD acceleration (SSE2/NEON)
  - AEAD encryption: 3.2 GB/s (XChaCha20-Poly1305)
  - BLAKE3 hashing: 8.5 GB/s with rayon parallelization and SIMD
  - File chunking: >1.5 GiB/s sequential read
  - Tree hashing: >3 GiB/s in-memory, ~2.5 GiB/s from disk
  - Chunk verification: <1μs per 256 KiB chunk
  - Missing chunks query: O(m) where m = missing count (was O(n))
- **Documentation:** 60+ files, 45,000+ lines including USER_GUIDE.md, CONFIG_REFERENCE.md, complete API documentation, architecture guides, deployment guides, security model, performance architecture
- **CI/CD:** GitHub Actions workflows for testing (Linux/macOS/Windows), security scanning (Dependabot, CodeQL, cargo-audit), multi-platform releases (6 targets: Linux x86_64/aarch64/musl, macOS Intel/ARM, Windows x86_64-msvc)
- **Security:** Zero vulnerabilities (cargo audit clean), CodeQL verified, weekly automated scans, RustSec advisory database integration, Gitleaks secret scanning
- **Code quality:** Zero clippy warnings, zero unsafe code in cryptographic paths, 50 unsafe blocks with 100% SAFETY documentation, constant-time operations for all cryptographic primitives

**Completed Components:**
- ✅ **Phase 1 (89 SP):** Frame encoding/decoding with SIMD acceleration (172M frames/sec), session state machine with 7 states, stream multiplexing with prioritization, BBR congestion control with bandwidth probing
- ✅ **Phase 2 (102 SP):** Ed25519 signatures with batch verification, X25519 key exchange with Elligator2 encoding, XChaCha20-Poly1305 AEAD with key commitment (3.2 GB/s), BLAKE3 hashing with SIMD (8.5 GB/s), Noise_XX handshake with mutual authentication, Double Ratchet with DH and symmetric ratcheting, replay protection with 64-bit sliding window
- ✅ **Phase 3 (156 SP):** AF_XDP zero-copy networking with UMEM, io_uring async I/O with registered buffers, UDP transport with SO_REUSEPORT, worker thread pools with CPU pinning and NUMA awareness, MTU discovery with binary search and caching
- ✅ **Phase 4 Part I (76 SP):** AF_XDP batch processing (rx_batch/tx_batch), BBR pacing enforcement with timer-based transmission, io_uring registered buffers for zero-copy, frame validation hardening (reserved stream IDs, offset bounds, payload limits)
- ✅ **Phase 4 Part II (167 SP):** Complete traffic obfuscation layer - PaddingEngine (5 modes: PowerOfTwo, SizeClasses, ConstantRate, Statistical), TimingObfuscator (5 distributions: Fixed, Uniform, Normal, Exponential), TLS 1.3 record layer mimicry, WebSocket binary framing (RFC 6455), DNS-over-HTTPS tunneling, adaptive threat-level profiles (Low/Medium/High/Paranoid)
- ✅ **Phase 5 (123 SP):** Discovery & NAT Traversal - Transport trait abstraction (AsyncUdpTransport), privacy-enhanced Kademlia DHT with BLAKE3 NodeIds and k-bucket routing (k=20), S/Kademlia Sybil resistance (20-bit difficulty, ~1M hash attempts), DHT privacy with BLAKE3-keyed info_hash, STUN client (RFC 5389) with MESSAGE-INTEGRITY authentication and NAT type detection, ICE candidate gathering with UDP hole punching, DERP-style relay infrastructure (RelayClient, RelayServer, RelaySelector with 4 selection strategies), unified DiscoveryManager orchestrating DHT/NAT/relay with automatic fallback
- ✅ **Phase 6 (98 SP):** Integration & File Transfer - Enhanced file chunking (FileChunker/FileReassembler with seek support, out-of-order writes, resume tracking with HashSet), BLAKE3 tree hashing with Merkle verification (compute_tree_hash, compute_merkle_root, verify_chunk, >3 GiB/s throughput), incremental tree hasher for streaming (zero-copy chunk boundaries), transfer session state machine (7 states, progress tracking, multi-peer coordination with chunk assignment, speed/ETA calculation), CLI implementation (send/receive/daemon/status/peers/keygen commands, progress display with indicatif, TOML configuration system with 6 sections), integration test framework (19 tests including end-to-end transfer with resume), performance benchmarks (chunking, tree hashing, verification, reassembly)
- ✅ **Phase 7 (158 SP):** Hardening & Optimization - Security audit with comprehensive review checklist, fuzzing infrastructure (5 libFuzzer targets: frame_parser, dht_message, padding, crypto, tree_hash), property-based testing (29 proptest invariants), O(m) missing chunks algorithm (was O(n), critical for large file resume), allocation-free incremental hashing, profiling infrastructure (CPU/memory/cache profiling with perf/valgrind), comprehensive documentation (USER_GUIDE.md ~800 lines, CONFIG_REFERENCE.md ~650 lines, expanded deployment guide with security hardening), cross-platform CI testing (Linux/macOS/Windows), packaging (deb/rpm/tar.gz with systemd service and security directives)
- ✅ **v0.8.0 Enhancements (52 SP):** 7 integration tests (end-to-end file transfer with 5MB resume, multi-peer coordination with 3 peers and 20 chunks, NAT traversal, relay fallback, obfuscation integration, Noise_XX + ratcheting), private key encryption at rest (Argon2id key derivation with OWASP-recommended defaults, XChaCha20-Poly1305 AEAD, passphrase rotation, security presets: low/default/high, 705 LOC with 16 tests), AEAD module refactoring (split 1,529 LOC into 4 focused modules: cipher.rs, replay.rs, session.rs for improved maintainability), BLAKE3 SIMD acceleration (rayon + neon features for 2-4x faster parallel hashing, ARM64 optimization), security audit template (comprehensive 10-section review checklist covering crypto/memory/side-channels/network/dependencies, penetration testing scope, fuzzing commands)
- ✅ **Phase 9 (85 SP):** Node API & Protocol Orchestration - Complete integration layer coordinating all protocol components (~4,000 lines, 9 modules, 57 tests). Sprint 9.1 (34 SP): Node struct with lifecycle, Identity management, session establishment, file transfer coordination, comprehensive configuration system. Sprint 9.2 (21 SP): DHT integration (announce, lookup_peer, find_peers, bootstrap), NAT traversal (STUN detection, ICE-lite hole punching, relay fallback), connection lifecycle (health monitoring, session migration). Sprint 9.3 (13 SP): Traffic obfuscation (4 padding modes, 4 timing distributions, 3 protocol mimicry types). Sprint 9.4 (17 SP): Multi-peer downloads with parallel chunk fetching, 7 integration tests, 4 performance benchmarks
- ✅ **Advanced Features:** Path MTU Discovery with binary search and caching, Connection Migration with PATH_CHALLENGE/RESPONSE, Cover Traffic Generation with Poisson/uniform distributions, Buffer Pools with pre-allocated UMEM, XDP packet filtering (planned), 15 documented frame types (DATA, ACK, CONTROL, REKEY, PING/PONG, CLOSE, PAD, STREAM_*, PATH_*)
- ✅ **Comprehensive test suite:** 791+ tests total (722 library + 40 integration + 29 property), 100% pass rate
- ✅ **Performance benchmarks:** 28 Criterion benchmarks measuring all critical paths
- ✅ **Security documentation:** SECURITY.md, comprehensive technical debt analysis

## Features

### Node API (v0.9.0)

**High-Level Protocol Orchestration:**
- **Node Lifecycle:** `Node::new_random()`, `start()`, `stop()` for node management
- **Session Management:** Noise_XX handshake with automatic key exchange
- **File Transfer:** `send_file()`, `receive_file()` with progress monitoring
- **DHT Integration:** Peer discovery, announcements, and lookup via Kademlia
- **NAT Traversal:** STUN detection, ICE-lite hole punching, relay fallback
- **Connection Management:** Health monitoring, session migration, automatic cleanup
- **Traffic Obfuscation:** Integrated padding, timing, and protocol mimicry
- **Multi-Peer Downloads:** Parallel chunk fetching with round-robin assignment
- **Comprehensive Configuration:** 6 subsystems (Transport, Obfuscation, Discovery, Transfer, Logging)

**Architecture:**
- **9 Modules:** node, config, session, error, discovery, nat, connection, obfuscation, transfer
- **Thread-Safe:** `Arc<RwLock<>>` shared state, `AtomicBool` lifecycle
- **~4,000 Lines:** Complete integration layer coordinating all protocol components
- **57 Tests:** Full coverage of all Node API operations

### Performance
- **Wire-Speed Transfers**: 10+ Gbps throughput with AF_XDP kernel bypass
- **Sub-Millisecond Latency**: <1ms packet processing with io_uring
- **Zero-Copy I/O**: Direct NIC-to-application data path via AF_XDP UMEM
- **Batch Processing**: rx_batch/tx_batch APIs for efficient packet handling
- **BBR Congestion Control**: Optimal bandwidth utilization with timer-based pacing
- **Async File I/O**: io_uring with registered buffers for zero-copy file operations

### Security

**Core Security Features:**
- **Ed25519 Digital Signatures**: Identity verification and message authentication
- **Strong Encryption**: XChaCha20-Poly1305 AEAD with key commitment (256-bit security, 192-bit nonce)
- **Key Exchange**: X25519 with Elligator2 encoding for indistinguishability
- **Perfect Forward Secrecy**: Double Ratchet with DH and symmetric ratcheting
- **Mutual Authentication**: Noise_XX handshake pattern (3-message mutual auth)
- **Hashing**: BLAKE3 with HKDF for key derivation

**Advanced Security:**
- **Replay Protection**: 64-bit sliding window bitmap prevents duplicate packet acceptance
- **Key Commitment for AEAD**: BLAKE3-based commitment prevents multi-key attacks
- **Automatic Rekey**: Configurable thresholds (90% default) for time, packets, and bytes
- **Constant-Time Operations**: All cryptographic operations timing side-channel resistant
- **Memory Safety**: Pure Rust implementation with ZeroizeOnDrop on all secret key material
- **Documented Unsafe Code**: Zero unsafe in crypto paths; performance-critical unsafe fully documented with SAFETY comments
- **S/Kademlia Sybil Resistance**: Crypto puzzle-based NodeId generation (20-bit difficulty, ~1M hash attempts)
- **DHT Privacy Enhancement**: BLAKE3-keyed info_hash prevents real content hash exposure
- **STUN MESSAGE-INTEGRITY**: RFC 5389 HMAC-SHA1 authentication with rate limiting (10 req/s default)

**v0.8.0 Security Enhancements:**
- **Private Key Encryption at Rest** (SEC-001, 705 LOC, 16 tests):
  - Argon2id key derivation with OWASP-recommended parameters (m=19456, t=2, p=1)
  - XChaCha20-Poly1305 AEAD encryption for private keys with 192-bit nonce
  - `EncryptedPrivateKey` with compact binary serialization (version + salt + nonce + ciphertext + tag)
  - `DecryptedPrivateKey` wrapper with automatic ZeroizeOnDrop on sensitive data
  - Passphrase rotation via `change_passphrase()` without key re-generation
  - Security presets: `low_security()` (m=4096, t=1), `default()` (OWASP), `high_security()` (m=65536, t=4)
  - Prevents key material exposure in memory dumps and swap files
- **Modular AEAD Architecture** (REFACTOR-001):
  - Refactored 1,529 LOC monolithic `aead.rs` into 4 focused modules (1,251 LOC total)
  - `aead/cipher.rs` (488 LOC): Core AEAD primitives (Nonce, Tag, AeadKey, AeadCipher)
  - `aead/replay.rs` (264 LOC): Replay protection with 64-bit sliding window
  - `aead/session.rs` (457 LOC): Session-level crypto with BufferPool
  - Improved maintainability and testability with zero behavior changes
- **BLAKE3 SIMD Acceleration** (PERF-001):
  - Enabled `rayon` feature for parallel tree hashing (2-4x speedup)
  - Enabled `neon` feature for ARM64 SIMD optimization
  - Throughput: 8.5 GB/s on x86_64 (AVX2), 6.2 GB/s on ARM64 (NEON)
- **Comprehensive Security Audit Template** (DOC-004):
  - 10-section review checklist: crypto/memory/side-channels/network/auth/input/dependencies/logging/fuzzing/pen-testing
  - Specific verification commands for constant-time operations, ZeroizeOnDrop, unsafe blocks
  - Penetration testing scope with attack scenarios
  - Fuzzing and sanitizer command reference

### Privacy & Obfuscation

**Traffic Analysis Resistance:**
- **Elligator2 Key Encoding**: X25519 public keys indistinguishable from random bytes
- **Packet Padding**: 5 modes (None, PowerOfTwo, SizeClasses, ConstantRate, Statistical)
  - PowerOfTwo: Round to next power of 2 (~15% overhead)
  - SizeClasses: Fixed size buckets [128, 512, 1024, 4096, 8192, 16384] (~10% overhead)
  - ConstantRate: Always maximum size (~50% overhead, maximum privacy)
  - Statistical: Geometric distribution-based random padding (~20% overhead)
- **Timing Obfuscation**: 5 distributions (None, Fixed, Uniform, Normal, Exponential)
  - Uniform: Random delays within configurable range
  - Normal: Gaussian distribution with mean and standard deviation
  - Exponential: Poisson process simulation for natural traffic patterns
- **Cover Traffic**: Constant, Poisson, and uniform distribution modes

**Protocol Mimicry:**
- **TLS 1.3 Record Layer**: Authentic-looking TLS application data records
  - Content type 23 (application_data), version 0x0303
  - Fake handshake generation (ClientHello, ServerHello, Finished)
  - Sequence number tracking for realistic sessions
- **WebSocket Binary Frames**: RFC 6455 compliant framing
  - Binary frame encoding with FIN bit and opcode 0x02
  - Client masking with random masking keys
  - Extended length encoding (126 for 16-bit, 127 for 64-bit)
- **DNS-over-HTTPS Tunneling**: Payload embedding in DNS queries
  - base64url encoding for query parameters
  - EDNS0 OPT records for payload carrier
  - Query/response packet construction and parsing

**Adaptive Obfuscation:**
- Threat-level-based profile selection (Low, Medium, High, Paranoid)
- Automatic mode selection based on operational context
- Configurable per-session obfuscation strategies

### Decentralization & Discovery

**Privacy-Enhanced Kademlia DHT:**
- **BLAKE3-based NodeId**: 256-bit cryptographic node identifiers
- **K-bucket Routing Table**: XOR-distance-based routing with k=20
- **Peer Discovery**: FIND_NODE queries with distance-based routing
- **Value Storage**: STORE and FIND_VALUE operations for peer announcements
- **S/Kademlia Sybil Resistance**: Crypto puzzle-based NodeId generation (20-bit difficulty)
  - O(1) verification, O(2^difficulty) generation (~1M hash attempts)
  - Protects DHT from Sybil and Eclipse attacks
- **DHT Privacy Enhancement**: BLAKE3-keyed `info_hash` computation
  - Real file hashes never exposed in DHT lookups
  - Only participants with `group_secret` can derive lookup keys
  - Privacy-preserving peer discovery

**NAT Traversal:**
- **STUN Client**: RFC 5389 compliant NAT type detection
  - Full Cone, Restricted Cone, Port-Restricted Cone, Symmetric NAT detection
  - Public IP and port mapping discovery
  - Multiple STUN server support for reliability
  - MESSAGE-INTEGRITY authentication (HMAC-SHA1) for secure STUN requests
  - Transaction ID validation and CRC-32 fingerprint verification
  - Rate limiting (10 req/s per IP default) for DoS protection
- **ICE-like Candidate Gathering**: Host, Server Reflexive, Relayed candidates
- **UDP Hole Punching**: Simultaneous open for NAT traversal
- **Relay Fallback**: Automatic relay selection when direct connection fails

**DERP-style Relay Infrastructure:**
- **RelayClient**: Connect to relay servers, packet forwarding, keepalive
- **RelayServer**: Multi-client support, packet routing, rate limiting
- **RelaySelector**: Intelligent relay selection with latency tracking
  - Selection strategies: LowestLatency, LowestLoad, HighestPriority, Balanced
  - Geographic region filtering
  - Load balancing across relays

**Unified Connection Flow:**
- **DiscoveryManager**: Orchestrates DHT, NAT traversal, and relay infrastructure
- **Connection Types**: Direct, HolePunched, Relayed
- **Automatic Fallback**: DHT lookup → Direct connection → Hole punch → Relay
- **Connection Migration**: Seamless IP address changes with PATH_CHALLENGE/PATH_RESPONSE

## Installation

### Pre-Built Binaries (Recommended)

Download pre-built binaries for your platform from the [releases page](https://github.com/doublegate/WRAITH-Protocol/releases):

**Supported Platforms:**
- Linux x86_64 (glibc and musl)
- Linux aarch64
- macOS x86_64 (Intel)
- macOS aarch64 (Apple Silicon)
- Windows x86_64

```bash
# Linux/macOS
tar xzf wraith-<platform>.tar.gz
chmod +x wraith
./wraith --version

# Windows (PowerShell)
Expand-Archive wraith-x86_64-windows.zip
.\wraith.exe --version
```

All release artifacts include SHA256 checksums for verification.

### Build From Source

**Prerequisites:**
- Rust 1.85+ (Rust 2024 edition)
- Linux 6.2+ (recommended for AF_XDP and io_uring support)
- x86_64 or aarch64 architecture

```bash
# Clone the repository
git clone https://github.com/doublegate/WRAITH-Protocol.git
cd WRAITH-Protocol

# Build all crates
cargo build --release

# Run tests
cargo test --workspace

# The wraith binary will be in target/release/wraith
./target/release/wraith --version
```

## Quick Start

**Note:** WRAITH Protocol is currently in early development (v0.1.0). The CLI interface is scaffolded but not yet functional. The following commands represent the planned interface:

```bash
# Send a file (coming soon)
wraith send document.pdf alice@peer.key

# Receive files (coming soon)
wraith receive --output ./downloads

# Run as daemon (coming soon)
wraith daemon --bind 0.0.0.0:0

# Generate a keypair (coming soon)
wraith keygen --output ~/.wraith/identity.key
```

For current development status, see [ROADMAP.md](to-dos/ROADMAP.md) and [Phase 1 Sprint Plan](to-dos/protocol/phase-1-foundation.md).

![WRAITH Protocol Architecture](images/wraith-protocol_arch-infographic.jpg)

## Project Structure

```
WRAITH-Protocol/
├── crates/                      # Rust workspace crates
│   ├── wraith-core/            # Frame encoding, sessions, congestion control
│   ├── wraith-crypto/          # Noise handshake, AEAD, Elligator2, ratcheting
│   ├── wraith-transport/       # AF_XDP, io_uring, UDP sockets
│   ├── wraith-obfuscation/     # Padding, timing, cover traffic, mimicry
│   ├── wraith-discovery/       # DHT, relay, NAT traversal
│   ├── wraith-files/           # Chunking, integrity, transfer state
│   ├── wraith-cli/             # Command-line interface
│   └── wraith-xdp/             # eBPF/XDP programs (Linux-only)
├── docs/                        # Comprehensive documentation
│   ├── architecture/           # Protocol design (5 docs)
│   ├── engineering/            # Development guides (4 docs)
│   ├── integration/            # Embedding & platform support (3 docs)
│   ├── testing/                # Testing strategies (3 docs)
│   ├── operations/             # Deployment & monitoring (3 docs)
│   └── clients/                # Client application docs (37 docs)
│       ├── overview.md         # Client ecosystem overview
│       ├── wraith-transfer/    # P2P file transfer (3 docs)
│       ├── wraith-chat/        # E2EE messaging (3 docs)
│       ├── wraith-sync/        # Backup sync (3 docs)
│       ├── wraith-share/       # File sharing (3 docs)
│       ├── wraith-stream/      # Media streaming (3 docs)
│       ├── wraith-mesh/        # IoT networking (3 docs)
│       ├── wraith-publish/     # Publishing (3 docs)
│       ├── wraith-vault/       # Secret storage (3 docs)
│       ├── wraith-recon/       # Security testing (6 docs)
│       └── wraith-redops/      # Red team ops (6 docs)
├── to-dos/                      # Sprint planning
│   ├── protocol/               # 7 implementation phases
│   ├── clients/                # 10 client application sprints
│   ├── ROADMAP.md              # Project roadmap
│   └── ROADMAP-clients.md      # Comprehensive client roadmap
├── ref-docs/                    # Technical specifications
└── xtask/                       # Build automation
```

## Client Applications

WRAITH Protocol powers a comprehensive ecosystem of secure applications across 3 priority tiers:

### Tier 1: Core Applications (High Priority)

| Client | Description | Status | Story Points |
|--------|-------------|--------|--------------|
| **WRAITH-Transfer** | Direct P2P file transfer with drag-and-drop GUI | Planned | 102 |
| **WRAITH-Chat** | E2EE messaging with Double Ratchet algorithm | Planned | 162 |

### Tier 2: Specialized Applications (Medium Priority)

| Client | Description | Status | Story Points |
|--------|-------------|--------|--------------|
| **WRAITH-Sync** | Decentralized backup synchronization (Dropbox alternative) | Planned | 136 |
| **WRAITH-Share** | Distributed anonymous file sharing (BitTorrent-like) | Planned | 123 |

### Tier 3: Advanced Applications (Lower Priority)

| Client | Description | Status | Story Points |
|--------|-------------|--------|--------------|
| **WRAITH-Stream** | Secure media streaming with live/VOD support (AV1/Opus) | Planned | 71 |
| **WRAITH-Mesh** | IoT mesh networking for decentralized device communication | Planned | 60 |
| **WRAITH-Publish** | Censorship-resistant publishing platform (blogs, wikis) | Planned | 76 |
| **WRAITH-Vault** | Distributed secret storage using Shamir Secret Sharing | Planned | 94 |

### Tier 3: Security Testing (Specialized - Authorized Use Only)

| Client | Description | Status | Story Points |
|--------|-------------|--------|--------------|
| **WRAITH-Recon** | Network reconnaissance & data exfiltration assessment | Planned | 55 |
| **WRAITH-RedOps** | Red team operations platform with C2 infrastructure | Planned | 89 |

**Total Ecosystem:** 10 clients, 1,028 story points, ~70 weeks development timeline.

**Security Testing Notice:** WRAITH-Recon and WRAITH-RedOps require signed authorization and governance compliance. See [Security Testing Parameters](ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md) for authorized use requirements.

See [Client Documentation](docs/clients/overview.md) and [Client Roadmap](to-dos/ROADMAP-clients.md) for comprehensive details.

## Development

### Prerequisites

- **Rust 1.85+** (Rust 2024 edition) - [Install Rust](https://www.rust-lang.org/tools/install)
- **Linux 6.2+** (recommended for AF_XDP and io_uring support)
- **x86_64 or aarch64** architecture
- **clang/LLVM** (optional, for XDP/eBPF compilation)

**Note:** While Linux 6.2+ is recommended for optimal performance with kernel bypass features, WRAITH Protocol includes UDP fallback that works on all platforms.

### Build Commands

```bash
# Development build
cargo build --workspace

# Release build with optimizations
cargo build --release

# Run all tests
cargo test --workspace

# Run lints
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt --all

# Run all CI checks (test + clippy + fmt + doc)
cargo xtask ci

# Generate API documentation
cargo doc --workspace --open

# Run benchmarks (coming soon)
cargo bench --workspace
```

### Cargo Aliases

WRAITH provides convenient cargo aliases (see `.cargo/config.toml`):

```bash
# Run full CI suite
cargo xtci

# Build and open documentation
cargo xtdoc

# Build XDP programs (Linux only, requires eBPF toolchain)
cargo xdbuild
```

### Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# Property-based tests
cargo test --features proptest

# Run with coverage
cargo tarpaulin --workspace --out Html
```

### Python Tooling (Optional)

WRAITH Protocol uses Python for auxiliary tasks like YAML linting. A Python virtual environment is provided:

```bash
# Quick health check (commands must be chained with &&)
source .venv/bin/activate && yamllint --version

# Lint GitHub Actions workflows
source .venv/bin/activate && yamllint .github/

# Automated venv setup/repair
bash scripts/venv-setup.sh
```

See [Python Tooling Guide](docs/engineering/python-tooling.md) for detailed documentation.

**Note:** Due to Claude Code's shell behavior, always chain commands with `&&` when using the venv.

## Documentation

### Getting Started
- [User Guide](docs/USER_GUIDE.md) - Installation, quick start, CLI reference
- [Configuration Reference](docs/CONFIG_REFERENCE.md) - Complete TOML configuration

### Architecture & Design
- [Protocol Overview](docs/architecture/protocol-overview.md)
- [Layer Design](docs/architecture/layer-design.md)
- [Security Model](docs/architecture/security-model.md)
- [Performance Architecture](docs/architecture/performance-architecture.md)
- [Network Topology](docs/architecture/network-topology.md)

### Development
- [Development Guide](docs/engineering/development-guide.md)
- [Coding Standards](docs/engineering/coding-standards.md)
- [API Reference](docs/engineering/api-reference.md)
- [Dependency Management](docs/engineering/dependency-management.md)
- [Python Tooling Guide](docs/engineering/python-tooling.md)

### Integration
- [Embedding Guide](docs/integration/embedding-guide.md)
- [Platform Support](docs/integration/platform-support.md)
- [Interoperability](docs/integration/interoperability.md)

### Testing & Operations
- [Testing Strategy](docs/testing/testing-strategy.md)
- [Performance Benchmarks](docs/testing/performance-benchmarks.md)
- [Deployment Guide](docs/operations/deployment-guide.md)
- [Monitoring](docs/operations/monitoring.md)

### Specifications
- [Protocol Technical Details](ref-docs/protocol_technical_details.md)
- [Implementation Guide](ref-docs/protocol_implementation_guide.md)

### Client Applications
- [Client Overview](docs/clients/overview.md)
- [Client Roadmap](to-dos/ROADMAP-clients.md)
- Individual client documentation (architecture, features, implementation, integration, testing, usage)

### Project Planning
- [Project Roadmap](to-dos/ROADMAP.md)
- [Client Roadmap](to-dos/ROADMAP-clients.md)
- [Documentation Status](docs/DOCUMENTATION_STATUS.md)

### Technical Debt & Quality
- [Technical Debt Analysis](to-dos/technical-debt/technical-debt-analysis.md) - Comprehensive code quality assessment
- [Technical Debt Action Plan](to-dos/technical-debt/technical-debt-action-plan.md) - Prioritized remediation strategy
- [Technical Debt TODO List](to-dos/technical-debt/technical-debt-todo-list.md) - Actionable tracking checklist
- [Pre-Phase 5 Review Summary](to-dos/technical-debt/pre-phase-5-review-summary.md) - Phase 5 readiness assessment (15 items analyzed)
- [Implementation Report](to-dos/technical-debt/IMPLEMENTATION-REPORT.md) - Detailed findings and recommendations
- [Phase 4 Technical Debt](to-dos/technical-debt/phase-4-tech-debt.md) - Phase 4 technical debt tracking
- **Current Metrics:** Grade A (92/100), 14% debt ratio, 607 tests, zero blocking items for Phase 5

### Security Testing
- [Security Testing Parameters](ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)
- [WRAITH-Recon Documentation](docs/clients/wraith-recon/)
- [WRAITH-RedOps Documentation](docs/clients/wraith-redops/)

## Roadmap

WRAITH Protocol development follows a structured 7-phase approach spanning 32-44 weeks:

### Protocol Development (947 Story Points)

| Phase | Focus | Duration | Story Points | Status |
|-------|-------|----------|--------------|--------|
| **Phase 1** | Foundation & Core Types | 4-6 weeks | 89 | ✅ **Complete** |
| **Phase 2** | Cryptographic Layer | 4-6 weeks | 102 | ✅ **Complete** |
| **Phase 3** | Transport & Kernel Bypass | 6-8 weeks | 156 | ✅ **Complete** |
| **Phase 4** | Optimization & Hardening (Part I) | 2-3 weeks | 76 | ✅ **Complete** |
| **Phase 5** | Discovery & NAT Traversal | 5-7 weeks | 123 | ✅ **Complete** |
| **Phase 6** | Integration & Testing | 4-5 weeks | 98 | ✅ **Complete** |
| **Phase 7** | Hardening & Optimization | 6-8 weeks | 158 | ✅ **Complete** |

**Progress:** 802/947 story points delivered (85% complete)

### Client Applications (1,028 Story Points)

10 client applications across 3 priority tiers, including:
- **Tier 1:** WRAITH-Transfer (P2P file transfer), WRAITH-Chat (E2EE messaging)
- **Tier 2:** WRAITH-Sync (backup sync), WRAITH-Share (distributed sharing)
- **Tier 3:** WRAITH-Stream, WRAITH-Mesh, WRAITH-Publish, WRAITH-Vault
- **Security Testing:** WRAITH-Recon, WRAITH-RedOps (authorized use only)

See [ROADMAP.md](to-dos/ROADMAP.md) and [Client Roadmap](to-dos/ROADMAP-clients.md) for detailed planning.

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Throughput (10 GbE) | >9 Gbps | AF_XDP with zero-copy |
| Throughput (1 GbE) | >950 Mbps | With encryption |
| Handshake Latency | <50 ms | LAN conditions |
| Packet Latency | <1 ms | NIC to application |
| Memory per Session | <10 MB | Including buffers |
| CPU @ 10 Gbps | <50% | 8-core system |

## CI/CD Infrastructure

WRAITH Protocol uses comprehensive automated workflows for quality assurance and releases:

### Continuous Integration
- **Testing:** Automated test suite on every push and pull request
- **Code Quality:** Clippy linting and rustfmt formatting checks
- **Documentation:** Automated doc generation and link validation
- **MSRV:** Minimum Supported Rust Version (1.85) verification

### Security Scanning
- **Dependabot:** Automated dependency updates with security prioritization
- **CodeQL:** Static analysis for security vulnerabilities
- **cargo-audit:** RustSec advisory database scanning
- **Gitleaks:** Secret scanning with false positive suppression
- **Weekly Scans:** Automated security checks every Monday

### Release Automation
- **Multi-Platform Builds:** 6 platform targets (Linux x86_64/aarch64, macOS Intel/ARM, Windows)
- **Artifact Generation:** Automated binary builds with SHA256 checksums
- **GitHub Releases:** Automatic release creation from version tags
- **Changelog Integration:** Automated release notes from CHANGELOG.md

See [CI Workflow](.github/workflows/ci.yml), [CodeQL Workflow](.github/workflows/codeql.yml), and [Release Workflow](.github/workflows/release.yml) for configuration details.

## Security

WRAITH Protocol is designed with security as a core principle:

### Cryptographic Suite

| Function | Algorithm | Security Level | Features |
|----------|-----------|----------------|----------|
| **Signatures** | Ed25519 | 128-bit | Identity verification, ZeroizeOnDrop |
| **Key Exchange** | X25519 | 128-bit | ECDH on Curve25519 |
| **Key Encoding** | Elligator2 | Traffic analysis resistant | Indistinguishable from random |
| **AEAD** | XChaCha20-Poly1305 | 256-bit key, 192-bit nonce | Key-committing, constant-time |
| **Hash** | BLAKE3 | 128-bit collision resistance | Tree-parallelizable, faster than SHA-3 |
| **KDF** | HKDF-BLAKE3 | 128-bit | Context-separated key derivation |
| **Handshake** | Noise_XX_25519_ChaChaPoly_BLAKE2s | Mutual auth | Identity hiding, forward secrecy |
| **Ratcheting** | Double Ratchet | Forward & post-compromise security | Symmetric per-packet + DH periodic |
| **Replay Protection** | 64-bit sliding window | DoS resistant | Constant-time bitmap operations |

### Security Features

**Cryptographic Guarantees:**
- **Forward Secrecy:** Double Ratchet with independent symmetric and DH ratchets
- **Post-Compromise Security:** DH ratchet heals from key compromise
- **Replay Protection:** 64-bit sliding window bitmap with constant-time operations
- **Key Commitment:** BLAKE3-based AEAD key commitment prevents multi-key attacks
- **Automatic Rekey:** Time-based (90% threshold), packet-count-based, byte-count-based triggers

**Traffic Analysis Resistance:**
- **Elligator2 Key Encoding:** X25519 public keys indistinguishable from random
- **Cover Traffic Generation:** Constant, Poisson, and uniform distribution modes
- **Padding:** Configurable padding modes for traffic shape obfuscation
- **Protocol Mimicry:** TLS, WebSocket, DNS-over-HTTPS wrappers

**Implementation Security:**
- **Memory Safety:** Rust with zero unsafe code in cryptographic paths
- **ZeroizeOnDrop:** Automatic zeroization of all secret key material
- **Constant-Time Operations:** Side-channel resistant implementations for all critical paths
- **SIMD Acceleration:** SSE2/NEON optimized frame parsing with security validation
- **Buffer Pools:** Pre-allocated buffers reduce allocation overhead without compromising security
- **Unsafe Code Audit:** 100% documentation coverage with SAFETY comments on all 40+ unsafe blocks
  - All `unsafe impl Send/Sync` implementations documented and justified
  - Thread safety analysis for kernel bypass operations
  - Safety invariants documented for UMEM, io_uring, CPU affinity operations

**Validation:**
- **Test Coverage:** 943 tests covering all protocol layers and security-critical paths
- **Integration Tests:** 117 integration and benchmark tests validating end-to-end workflows
- **Cryptographic Tests:** 123 tests for Ed25519, X25519, Elligator2, AEAD, Noise_XX, Double Ratchet
- **Obfuscation Tests:** 167 tests (130 unit + 37 doctests) for traffic analysis resistance
- **Fuzzing:** 5 libFuzzer targets continuously testing parsing robustness
- **Property-Based Tests:** 29 proptest invariants for state machine validation
- **Automated Security Scanning:** Dependabot, CodeQL, RustSec advisories, cargo-audit weekly scans

### Reporting Vulnerabilities

For security issues, please see [SECURITY.md](SECURITY.md) for our security policy and responsible disclosure process.

## Getting Involved

WRAITH Protocol is in active development and we welcome contributions of all kinds:

### For Developers
- **Phase 1 Implementation:** Help complete the core protocol foundation (session state machine, stream multiplexing)
- **Testing:** Write unit tests, integration tests, and property-based tests
- **Documentation:** Improve API docs, add examples, clarify specifications
- **Code Review:** Review pull requests and provide feedback

### For Security Researchers
- **Protocol Review:** Analyze cryptographic design and security properties
- **Penetration Testing:** Test implementations for vulnerabilities (coordinated disclosure)
- **Formal Verification:** Assist with formal proofs of security properties

### For Writers
- **Technical Writing:** Improve documentation clarity and completeness
- **Tutorials:** Create getting-started guides and usage examples
- **Translations:** Translate documentation to other languages

### Current Focus Areas
1. ✅ **Phase 1 Complete** - Core protocol foundation (197 tests, 172M frames/sec, SIMD acceleration)
2. ✅ **Phase 2 Complete** - Cryptographic layer (123 tests, full security suite with Ed25519)
3. ✅ **Phase 3 Complete** - Transport & kernel bypass (54 tests, AF_XDP, io_uring, worker pools, NUMA)
4. ✅ **Phase 4 Part I Complete** - Optimization & hardening (AF_XDP batch processing, BBR pacing, io_uring registered buffers, frame validation)
5. ✅ **Phase 4 Part II Complete** - Obfuscation & stealth (167 tests, 5 padding modes, 5 timing distributions, TLS/WebSocket/DoH mimicry, adaptive profiles)
6. ✅ **Phase 5 Complete** - Discovery & NAT traversal (184 tests, Kademlia DHT, STUN/ICE, relay infrastructure, unified DiscoveryManager)
7. ✅ **Phase 6 Complete** - Integration & file transfer (transfer sessions, BLAKE3 tree hashing, CLI implementation)
8. ✅ **Phase 7 Complete** - Security audit, fuzzing (5 targets), O(m) optimizations, comprehensive documentation, cross-platform packaging
9. ✅ **Advanced Security Features** - Replay protection, key commitment, automatic rekey, reserved stream ID validation, constant-time operations
10. ✅ **Performance Optimizations** - SIMD frame parsing, buffer pools, fixed-point BBR arithmetic, O(m) missing chunks, zero-copy batch processing
11. ✅ **Comprehensive Documentation** - USER_GUIDE.md, CONFIG_REFERENCE.md, expanded API reference, deployment guide
12. ✅ **Cross-Platform Packaging** - deb, rpm, tar.gz packages with systemd service
13. ✅ **Fuzzing & Property Testing** - 5 libFuzzer targets, 29 proptest invariants
14. **Next: Client Applications** - WRAITH-Transfer, WRAITH-Chat, and other protocol clients
15. Maintain test coverage (current: 943 tests, target: maintain 80%+ coverage)

See [ROADMAP.md](to-dos/ROADMAP.md) for detailed sprint planning and story point estimates.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for comprehensive guidelines.

### Quick Start for Contributors

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes with tests
4. Run CI checks locally (`cargo xtask ci`)
5. Commit your changes (`git commit -m 'feat: add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Contribution Requirements
- Follow Rust coding standards (rustfmt, clippy)
- Add tests for new functionality
- Update documentation (API docs, CHANGELOG.md)
- Sign commits (optional but encouraged)
- Follow [Conventional Commits](https://www.conventionalcommits.org/) format

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Acknowledgments

WRAITH Protocol builds on the work of many excellent projects and technologies:

### Protocol Inspirations
- [Noise Protocol Framework](https://noiseprotocol.org/) - Cryptographic handshake patterns
- [WireGuard](https://www.wireguard.com/) - Design philosophy: simplicity and performance
- [QUIC](https://quicwg.org/) - Connection migration and modern transport
- [libp2p](https://libp2p.io/) - DHT and NAT traversal patterns
- [Signal Protocol](https://signal.org/docs/) - Double ratchet algorithm

### Cryptographic Libraries
- [RustCrypto](https://github.com/RustCrypto) - ChaCha20-Poly1305, X25519, BLAKE3 implementations
- [Snow](https://github.com/mcginty/snow) - Noise Protocol Framework for Rust
- [dalek-cryptography](https://github.com/dalek-cryptography) - Ed25519 and X25519

### Performance Technologies
- [AF_XDP](https://www.kernel.org/doc/html/latest/networking/af_xdp.html) - Kernel bypass networking
- [io_uring](https://kernel.dk/io_uring.pdf) - Efficient async I/O
- [eBPF/XDP](https://ebpf.io/) - In-kernel packet processing

## Links

- **Repository:** [github.com/doublegate/WRAITH-Protocol](https://github.com/doublegate/WRAITH-Protocol)
- **Documentation:** [docs/](docs/)
- **Issue Tracker:** [GitHub Issues](https://github.com/doublegate/WRAITH-Protocol/issues)
- **Discussions:** [GitHub Discussions](https://github.com/doublegate/WRAITH-Protocol/discussions)
- **Security Policy:** [SECURITY.md](SECURITY.md)
- **Changelog:** [CHANGELOG.md](CHANGELOG.md)
- **Roadmap:** [ROADMAP.md](to-dos/ROADMAP.md)

---

**WRAITH Protocol** - *Secure. Fast. Invisible.*

**Status:** v0.9.0 Beta (Node API) | **License:** MIT | **Language:** Rust 2024 (MSRV 1.85) | **Tests:** 791+ (722 library + 40 integration + 29 property) | **Quality:** Grade A+ (95/100), 12% debt ratio, 0 vulnerabilities, 5 fuzz targets | **Protocol:** Phase 9 Complete - Node API Integration Layer (887/947 SP, 94%)
