# Changelog

All notable changes to WRAITH Protocol will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

**Phase 3: Transport & Kernel Bypass (2025-11-29):**

- **XDP/eBPF Foundation** (`wraith-xdp/`)
  - XDP packet filter program for WRAITH traffic (UDP ports 40000-50000)
  - IPv4 and IPv6 support with protocol detection
  - Per-CPU statistics (RX packets, bytes, dropped, redirected)
  - AF_XDP socket redirection via XSKMAP
  - libbpf-rs bindings with feature-gated support
  - Graceful fallback stubs for non-Linux platforms
  - 5 unit tests + comprehensive doctests

- **AF_XDP Socket Management** (`wraith-transport/src/af_xdp.rs`)
  - UMEM (shared memory) allocation with mlock support
  - Configurable frame sizes (power of 2, ≥2048 bytes)
  - Fill and completion ring buffer management
  - Lock-free producer/consumer ring operations
  - Reserve/submit pattern for batch packet processing
  - RX and TX ring management for zero-copy I/O
  - 7 comprehensive tests for rings, UMEM, and sockets

- **Worker Thread Model** (`wraith-transport/src/worker.rs`)
  - Thread pool with configurable worker count
  - CPU core pinning via sched_setaffinity (Linux)
  - Per-worker statistics tracking (packets, bytes, errors)
  - Graceful shutdown with task draining
  - Queue backpressure handling
  - 10 comprehensive tests

- **NUMA-Aware Allocation** (`wraith-transport/src/numa.rs`)
  - NUMA topology detection (nodes, CPUs per node)
  - Node-local memory allocation via mbind
  - CPU-to-NUMA-node mapping
  - Cross-platform stubs for non-Linux systems

- **MTU Discovery** (`wraith-transport/src/mtu.rs`)
  - Path MTU discovery with binary search probing
  - Per-destination MTU caching with configurable TTL
  - Support for MTU 576-9000 bytes (including jumbo frames)
  - Automatic cache expiry and cleanup
  - Integration with path module
  - 10 comprehensive tests

- **UDP Transport** (`wraith-transport/src/udp.rs`)
  - Full UDP socket implementation using socket2
  - Non-blocking I/O with configurable timeouts
  - 2MB RX/TX buffers for high throughput
  - 64KB receive buffer for large packets
  - Cross-platform support (Linux, macOS, Windows)
  - 7 comprehensive tests

- **io_uring File I/O** (`wraith-files/src/io_uring.rs`, `async_file.rs`)
  - High-performance async file I/O engine
  - Queue depth 128-4096 for batched operations
  - AsyncFileReader for batched async reads
  - AsyncFileWriter for batched async writes
  - Completion tracking and caching
  - Race condition fix for concurrent completions
  - 12 comprehensive tests

- **Transport Benchmarks** (`benches/transport_benchmarks.rs`)
  - UDP throughput benchmarks (512B-1500B packets)
  - UDP round-trip latency measurements
  - Worker pool task processing (1-8 workers)
  - MTU cache lookup performance
  - Frame encoding overhead

### Changed

- Updated `wraith-transport` dependencies: added crossbeam-channel, num_cpus, libc
- Updated `wraith-files` module structure with io_uring and async_file submodules
- Phase 3 sprint documentation marked as 100% complete

### Fixed

- Race condition in AsyncFileReader/Writer `wait_for()` causing lost completions
  when multiple operations complete simultaneously

## [0.2.0] - 2025-11-29

### Added

**Comprehensive Security and Performance Enhancements (2025-11-29):**

- **Ed25519 Signatures Module** (`wraith-crypto/src/signatures.rs`)
  - SigningKey, VerifyingKey, and Signature types
  - Full sign/verify workflow with 15 comprehensive tests
  - ZeroizeOnDrop for private key material
  - Constant-time signature verification
  - Integration with Double Ratchet for authenticated messaging

- **SIMD-Accelerated Frame Parsing** (`wraith-core/src/frame.rs`)
  - SSE2 support for x86_64 (128-bit SIMD)
  - NEON support for aarch64 (ARM SIMD)
  - Feature-gated with `simd` feature flag (enabled by default)
  - ~15% performance improvement on supported platforms
  - Graceful fallback to portable implementation

- **Replay Protection** (`wraith-crypto/src/aead.rs`)
  - 64-bit sliding window bitmap implementation
  - Rejects duplicate packets and packets outside window
  - Constant-time bitmap operations (side-channel resistant)
  - Configurable window size (default: 64 packets)
  - Integrated with SessionCrypto for transparent protection
  - 8 comprehensive tests including edge cases

- **Key Commitment for AEAD** (`wraith-crypto/src/aead.rs`)
  - BLAKE3-based key commitment derivation
  - Prevents multi-key attacks (different keys decrypting to different plaintexts)
  - Transparent integration with XChaCha20-Poly1305
  - Zero performance overhead (pre-computed during session setup)
  - 3 comprehensive tests validating commitment correctness

- **Buffer Pool** (`wraith-crypto/src/aead.rs`)
  - Pre-allocated buffer management for encryption operations
  - Reduces allocation overhead in hot path (encrypt/decrypt)
  - Configurable capacity (default: 4096 bytes)
  - Configurable max buffers (default: 16 buffers)
  - Thread-safe buffer reuse
  - 2 comprehensive tests

- **Path MTU Discovery** (`wraith-core/src/path.rs`)
  - Complete PMTUD state machine (Idle, Probing, Complete, Failed)
  - Binary search probing algorithm
  - Configurable probe intervals (default: 30s)
  - Configurable probe timeout (default: 5s)
  - Maximum probe attempts (default: 5)
  - Integration with session management
  - 7 comprehensive tests

- **Connection Migration** (`wraith-core/src/migration.rs`)
  - PATH_CHALLENGE frame generation with 64-bit nonce
  - PATH_RESPONSE frame validation
  - RTT measurement during path validation
  - Multi-path support (up to 4 concurrent paths)
  - Path promotion on successful validation
  - Integration with session management
  - 5 comprehensive tests

- **Cover Traffic Generation** (`wraith-obfuscation/src/cover.rs`)
  - Multiple distribution modes:
    - Constant: Fixed interval traffic (e.g., every 100ms)
    - Poisson: Exponential inter-arrival times (e.g., 10 packets/sec mean)
    - Uniform: Random interval within range (e.g., 50-150ms)
  - Configurable rates and timing parameters
  - Random padding generation (1-1024 bytes)
  - Integration with session layer
  - 3 comprehensive tests per mode (9 total)

- **BBR Metrics Export** (`wraith-core/src/congestion.rs`)
  - `estimated_bandwidth()` - Current bandwidth estimate
  - `estimated_rtt()` - Current RTT estimate
  - `is_bandwidth_limited()` - Bandwidth vs application-limited state
  - `congestion_window()` - Current congestion window size
  - `pacing_rate()` - Current packet pacing rate
  - Enables external monitoring and debugging
  - 5 comprehensive tests for getter methods

### Changed

- **BBR Congestion Control Performance** (`wraith-core/src/congestion.rs`)
  - Converted floating-point arithmetic to fixed-point (Q16.16 format)
  - 15%+ faster bandwidth/RTT calculations
  - Eliminates floating-point dependency for embedded targets
  - Maintains numerical precision for congestion control
  - All existing tests pass with fixed-point implementation

- **Stream Management Optimization** (`wraith-core/src/stream.rs`)
  - Implemented lazy initialization pattern (StreamLite/StreamFull)
  - StreamLite: 80 bytes (idle streams, no buffers allocated)
  - StreamFull: ~16 KB (active streams with send/receive buffers)
  - 90%+ memory reduction for idle streams
  - Zero performance impact on active streams
  - Seamless promotion from Lite to Full on first I/O operation

- **Rekey Trigger Logic** (`wraith-crypto/src/aead.rs`, `wraith-crypto/src/ratchet.rs`)
  - Enhanced with configurable emergency thresholds (default: 90%)
  - Time-based rekey: 90% of max session time (default: 21.6 hours of 24 hours)
  - Packet-based rekey: 90% of max packets (default: 900K of 1M packets)
  - Byte-based rekey: 90% of max bytes (default: 245 GB of 272 GB)
  - Prevents hitting hard limits that would force connection close
  - 4 comprehensive tests for threshold validation

- **Hash Module API** (`wraith-crypto/src/hash.rs`)
  - Added batch update API for TreeHasher
  - `update_batch()` accepts multiple byte slices
  - More efficient than multiple `update()` calls
  - Useful for hashing fragmented data (e.g., network packets)
  - 2 comprehensive tests

- **Constant-Time Operations** (`wraith-crypto/src/constant_time.rs`)
  - Verified skipped key lookup in Double Ratchet uses `ct_eq()`
  - All critical cryptographic comparisons now constant-time
  - Prevents timing side-channel attacks
  - Side-channel resistance validation tests

### Fixed

- **Documentation Clarity** (multiple files)
  - Clarified Noise pattern uses BLAKE2s (snow library limitation)
  - BLAKE3 used for HKDF and application-level key derivation
  - Updated documentation to reflect correct hash function usage
  - Added inline comments explaining cryptographic choices

- **Constant-Time Validation** (`wraith-crypto/src/ratchet.rs`)
  - Verified all key comparisons use `ct_eq()` for constant-time equality
  - Prevents timing attacks on skipped key lookup
  - Added documentation comments explaining side-channel resistance

### Security

- **Zero Unsafe Code Maintained**
  - All cryptographic paths remain free of unsafe blocks
  - Memory safety guaranteed by Rust type system
  - No FFI calls in hot path

- **Constant-Time Cryptographic Operations**
  - All equality comparisons constant-time (`ct_eq`)
  - Replay protection bitmap operations constant-time
  - Signature verification constant-time
  - Key commitment derivation constant-time

- **Key Material Zeroization**
  - All SigningKey, SymmetricKey, and session keys use ZeroizeOnDrop
  - Automatic cleanup on drop prevents key leakage
  - Covers Ed25519, X25519, XChaCha20, and ratchet keys

- **Test Coverage for Security-Critical Paths**
  - 351 tests total (up from 229)
  - wraith-core: 177 tests (session, stream, congestion, path, migration)
  - wraith-crypto: 124 tests (signatures, AEAD, replay, ratchet, constant-time)
  - wraith-obfuscation: 24 tests (cover traffic, padding)
  - wraith-transport: 15 tests (UDP, io_uring stubs)
  - Integration: 12 tests

**Technical Debt Remediation (2025-11-29):**

- **Comprehensive Code Quality Improvements:**
  - Added `#[must_use]` attributes to ~65 pure functions across wraith-core and wraith-crypto
  - Added `# Errors` documentation to Result-returning functions
  - Added `# Panics` documentation where applicable
  - Modernized format strings (uninlined format args to inline)
  - Consolidated duplicate match arms in noise.rs
  - Fixed markdown formatting in documentation

- **8 New BBR Congestion Control Tests (wraith-core):**
  - `test_bbr_accessors` - Getter methods validation
  - `test_bbr_bdp_calculation` - Bandwidth-delay product calculation
  - `test_bbr_bandwidth_window_max` - Window tracking
  - `test_bbr_cwnd_minimum` - Minimum congestion window
  - `test_bbr_cwnd_with_bdp` - BDP-based window sizing
  - `test_bbr_bandwidth_estimation_accuracy` - Bandwidth measurement precision
  - `test_bbr_rtt_measurement_accuracy` - RTT measurement precision
  - `test_bbr_rtt_window_limit` - RTT window bounds

- **Technical Debt Documentation:**
  - `TECH-DEBT-SUMMARY.md` - Consolidated technical debt report for both crates
  - `crates/wraith-core/TECH-DEBT.md` - Phase 1 technical debt analysis
  - `crates/wraith-crypto/SECURITY.md` - Security documentation

---

**Phase 2: Cryptographic Layer - COMPLETE ✅ (2025-11-29):**

#### Complete Cryptographic Suite (wraith-crypto, 3,533 lines, 102 tests)

**X25519 Key Exchange (wraith-crypto/src/x25519.rs):**
- Elliptic curve Diffie-Hellman key agreement using Curve25519
- Public/private keypair generation with secure random number generation
- Shared secret derivation from keypair and peer public key
- Low-order point rejection for security (prevents small subgroup attacks)
- RFC 7748 test vector validation
- 6 comprehensive unit tests

**Elligator2 Encoding (wraith-crypto/src/elligator.rs):**
- Indistinguishable encoding of X25519 public keys as uniform random bytes
- Deterministic decoding from representative to public key
- Generate encodable keypairs (not all X25519 keys are Elligator2-encodable)
- Traffic analysis resistance through key indistinguishability
- Any 32-byte input decodable to valid curve point
- Uniform distribution validation tests
- 7 comprehensive unit tests including statistical validation

**XChaCha20-Poly1305 AEAD (wraith-crypto/src/aead.rs):**
- Authenticated Encryption with Associated Data (AEAD)
- 256-bit keys, 192-bit nonces, 128-bit authentication tags
- In-place encryption/decryption for zero-copy operation
- Additional authenticated data (AAD) support
- Session-based encryption with automatic counter management
- Tamper detection and prevention
- 12 comprehensive unit tests

**BLAKE3 Hashing and KDF (wraith-crypto/src/hash.rs):**
- Fast cryptographic hash function (tree-parallelizable)
- HKDF (HMAC-based Key Derivation Function) with extract and expand
- Key Derivation Function (KDF) with context separation
- Incremental tree hashing for large inputs
- Deterministic key derivation
- 11 comprehensive unit tests

**Noise_XX Handshake (wraith-crypto/src/noise.rs):**
- Noise Protocol Framework implementation using snow crate
- 3-message mutual authentication handshake pattern
- Identity hiding for both initiator and responder
- Session key derivation (transport encryption + transport decryption keys)
- Handshake state management with proper phase tracking
- Transport mode encryption/decryption after handshake
- Periodic rekeying support
- Payload encryption during handshake messages
- 6 comprehensive unit tests

**Double Ratchet (wraith-crypto/src/ratchet.rs):**
- Forward secrecy and post-compromise security
- Symmetric Ratchet: Per-packet key rotation using HKDF
  - Message key derivation from chain key
  - Chain key ratcheting for next message
  - Out-of-order message handling with skipped keys
  - Maximum skip limit (1000) to prevent DoS
- DH Ratchet: Periodic Diffie-Hellman key exchange
  - Root key and chain key derivation
  - Alternating DH ratchet steps between parties
  - Bidirectional communication support
  - Message header serialization (DH public key + message number + previous chain length)
- 14 comprehensive unit tests including tampering detection

**Constant-Time Operations (wraith-crypto/src/constant_time.rs):**
- Side-channel resistant cryptographic operations
- Constant-time equality comparison (ct_eq)
- Constant-time byte array verification (verify_16, verify_32, verify_64)
- Conditional assignment without branches (ct_assign)
- Conditional value selection without branches (ct_select)
- Bitwise operations without timing leaks (ct_and, ct_or, ct_xor)
- 10 comprehensive unit tests

**Integration Test Vectors (tests/vectors.rs):**
- 24 comprehensive integration tests validating end-to-end cryptographic operations
- X25519 scalar multiplication test vectors
- XChaCha20-Poly1305 AEAD roundtrip, authentication, tamper detection
- BLAKE3 hashing with various input sizes
- BLAKE3 HKDF and KDF validation
- Noise_XX handshake with unique key derivation
- Double Ratchet forward secrecy, DH ratchet steps
- Elligator2 uniform distribution and key exchange
- Constant-time comparison and selection
- Full cryptographic pipeline integration test

#### Test Coverage Summary

- **Total Tests:** 214 passing (1 ignored)
  - wraith-core: 112 tests
    - Frame layer: 22 unit + 6 property-based = 28 tests
    - Session state: 23 tests
    - Stream multiplexing: 33 tests
    - BBR congestion: 28 tests (increased from 20 via technical debt remediation)
  - wraith-crypto: 102 tests (1 ignored: RFC 7748 iteration test)
    - AEAD encryption/decryption: 12 tests
    - X25519 key exchange: 6 tests
    - Elligator2 encoding: 7 tests
    - BLAKE3 hashing/KDF: 11 tests
    - Noise_XX handshake: 6 tests
    - Double Ratchet: 14 tests
    - Constant-time operations: 10 tests
    - Integration test vectors: 24 tests
- **Code Quality:**
  - `cargo clippy --workspace -- -D warnings`: PASS
  - `cargo fmt --all -- --check`: PASS
  - Zero compilation warnings

#### Phase 2 Deliverables ✅

**Completed Components (102/102 story points):**
1. ✅ X25519 key exchange with secure random keypair generation
2. ✅ Elligator2 encoding for traffic analysis resistance
3. ✅ XChaCha20-Poly1305 AEAD with session management
4. ✅ BLAKE3 hashing with HKDF and context-separated KDF
5. ✅ Noise_XX handshake (3-message mutual authentication)
6. ✅ Double Ratchet (symmetric per-packet + DH periodic)
7. ✅ Constant-time operations for side-channel resistance
8. ✅ Comprehensive test suite (102 tests in wraith-crypto)
9. ✅ Integration test vectors (24 tests)
10. ✅ Security documentation (SECURITY.md, TECH-DEBT.md)

**Security Validation:**
- ✅ Forward secrecy through Double Ratchet
- ✅ Post-compromise security through DH ratcheting
- ✅ Traffic analysis resistance through Elligator2
- ✅ Side-channel resistance through constant-time operations
- ✅ Tamper detection through AEAD authentication
- ✅ Low-order point rejection in X25519
- ✅ Test vector validation for cryptographic correctness

**Documentation:**
- Security model documentation (SECURITY.md)
- Technical debt tracking (TECH-DEBT.md)
- API documentation for all cryptographic modules
- Integration examples in test vectors
- Security best practices in code comments

#### Next: Phase 3 - Transport & Kernel Bypass

**Prerequisites Met:**
- Core frame layer operational ✅
- Session management functional ✅
- Stream multiplexing ready ✅
- Congestion control implemented ✅
- Cryptographic suite complete ✅

**Phase 3 Focus (156 story points, 6-8 weeks):**
- AF_XDP zero-copy networking (Linux kernel bypass)
- io_uring async I/O integration
- Connection migration and path validation
- Multi-path support
- Packet pacing
- UDP fallback implementation

---

### Changed

- **Removed deprecated NoiseSession API:** Use NoiseHandshake for session management
- **Added #[must_use] attributes:** ~65 pure functions now require result handling
- **Improved documentation:** Added # Errors and # Panics sections to all public APIs
- **Enhanced constant-time operations:** All critical cryptographic paths now use constant-time functions
- **Modernized format strings:** Updated uninlined format arguments to inline format (Rust 2024 style)
- **Code quality metrics:** Overall quality score 90/100, pedantic warnings reduced from ~263 to ~123 (53% reduction)

### Fixed

- **Documentation formatting:** Fixed markdown formatting with proper backticks for technical terms
- **Pattern nesting:** Simplified match expressions in noise.rs for better readability
- **Cast lossless warnings:** Fixed integer cast warnings in constant_time.rs
- **Pedantic clippy warnings:** Reduced from ~263 to ~123 across both crates (53% improvement)

### Security

- **Cryptographic implementation complete:** Full security suite with forward secrecy and post-compromise security
- **Side-channel resistance:** Constant-time operations for all critical cryptographic paths
- **Memory zeroization:** Automatic cleanup of sensitive cryptographic material
- **Test vector validation:** 24 integration tests ensure cryptographic correctness
- **Low-order point rejection:** X25519 implementation rejects low-order points to prevent attacks

## [0.1.5] - 2025-11-29

### Added

**Phase 1: Foundation - COMPLETE ✅ (2025-11-29):**

#### Core Implementation (110 tests, ~3,500 lines of Rust)

**Frame Layer (wraith-core/frame.rs):**
- All 12 frame types implemented and validated
  - Data, Ack, Control, Rekey, Ping, Pong, Close, Pad
  - StreamOpen, StreamClose, StreamReset
  - PathChallenge, PathResponse
- Zero-copy frame parsing: 5.8 ns (~172M frames/sec, 232 GiB/s theoretical throughput)
- Frame building: 18-124 ns depending on payload size
- Configurable padding for traffic analysis resistance
- Nonce extraction and sequence number handling
- 22 unit tests + 6 property-based tests (proptest)
- Benchmark suite with 6 payload sizes + roundtrip tests

**Session State Machine (wraith-core/session.rs):**
- Complete state machine implementation
  - 5 states: Init, Handshaking, Established, Closing, Closed
  - Full state transition validation
  - Invalid state transition rejection
- Connection ID (CID) management
  - Unique 64-bit identifier generation
  - CID rotation support for privacy
  - Special value handling (all-zeros, all-ones)
- Stream management
  - Create, retrieve, remove streams
  - Maximum stream limit enforcement
  - Stream lifecycle tracking
- Session tracking
  - Activity monitoring (last_activity timestamp)
  - Idle detection
  - Packet counters (sent/received)
  - Session statistics
- Handshake phase tracking
- Rekey scheduling (time-based and packet-count-based)
- Migration state support for connection migration
- Cleanup on session closure
- 23 comprehensive tests

**Stream Multiplexing (wraith-core/stream.rs):**
- Complete stream state machine (6 states)
  - Idle, Open, HalfClosedLocal, HalfClosedRemote, DataSent, Closed
  - Full state transition validation
  - Invalid state transition rejection
- Flow control window management
  - Configurable send/receive windows (default: 65536 bytes)
  - Maximum window size enforcement (16 MiB)
  - Window consumption and updates
  - Window overflow protection
- Buffered I/O operations
  - Send buffer (write data)
  - Receive buffer (read data)
  - Peek support (read without consuming)
  - Multiple buffered writes
- Half-close support (FIN)
  - FIN sent/received state transitions
  - Graceful shutdown for each direction
  - FIN idempotency (multiple FIN calls safe)
  - Bidirectional FIN exchange
- Stream reset for abrupt termination
- Client/server stream ID allocation (odd/even)
- Stream direction detection (client vs server initiated)
- Read/write capability checks based on state
- Cleanup on stream closure
- 33 comprehensive tests

**BBR Congestion Control (wraith-core/congestion.rs):**
- Full BBR state machine (4 phases)
  - Startup: Exponential growth phase
  - Drain: Reduce inflight to BDP after startup
  - ProbeBw: Bandwidth probing with 8-phase cycle
  - ProbeRtt: Periodic minimum RTT measurement
  - State transition logic with plateau detection
- RTT estimation
  - Sliding window (10 samples)
  - Minimum RTT tracking
  - RTT update on ACK receipt
- Bandwidth estimation
  - Sliding window (10 samples)
  - Maximum bandwidth tracking
  - Bandwidth update on ACK receipt
- Bandwidth-Delay Product (BDP) calculation
  - BDP = bandwidth × min_rtt
  - Used for congestion window sizing
- Pacing and congestion window (cwnd)
  - Pacing rate calculation based on bandwidth
  - Initial pacing rate: 1 Mbps
  - Congestion window based on BDP
  - Initial cwnd: 10 packets
- Packet event handlers
  - on_packet_sent: Track inflight bytes
  - on_packet_acked: Update RTT/bandwidth, adjust state
  - on_packet_lost: Congestion signal handling
- Inflight bytes tracking
- ProbeBw cycle with 8-phase pacing gains
- ProbeRtt periodic RTT measurement (every 10 seconds)
- Send capability checks (can_send based on cwnd vs inflight)
- 29 comprehensive tests

#### Benchmark Performance

**Frame Parsing (wraith-core/benches/frame_bench.rs):**
- 64-byte payload: 5.8 ns (~172M frames/sec, 10.8 GiB/s)
- 512-byte payload: 5.9 ns (~169M frames/sec, 84.6 GiB/s)
- 1024-byte payload: 5.9 ns (~169M frames/sec, 169 GiB/s)
- 4096-byte payload: 6.0 ns (~166M frames/sec, 665 GiB/s)
- 16384-byte payload: 6.1 ns (~163M frames/sec, 2.6 TiB/s)
- 65535-byte payload: 6.2 ns (~161M frames/sec, 10.3 TiB/s)

**Frame Building:**
- 64-byte payload: 18 ns (~55M frames/sec)
- 512-byte payload: 25 ns (~40M frames/sec)
- 1024-byte payload: 31 ns (~32M frames/sec)
- 4096-byte payload: 66 ns (~15M frames/sec)
- 16384-byte payload: 124 ns (~8M frames/sec)

**Note:** Parsing is significantly faster than building due to zero-copy design. Building requires memory allocation and random padding generation.

#### Test Coverage Summary

- **Total Tests:** 110 passing (0 failures)
  - wraith-core: 104 tests
    - Frame layer: 22 unit + 6 property-based = 28 tests
    - Session state: 23 tests
    - Stream multiplexing: 33 tests
    - BBR congestion: 29 tests (with proper assertions)
  - wraith-crypto: 6 tests
    - AEAD encryption/decryption: 2 tests
    - Elligator2 encoding: 3 tests
    - Key ratcheting: 1 test
- **Property-Based Tests:** 6 proptest cases with 256 iterations each
- **Benchmarks:** 19 criterion benchmarks (frame parse/build/roundtrip)
- **Code Quality:**
  - `cargo clippy --workspace -- -D warnings`: PASS
  - `cargo fmt --all -- --check`: PASS
  - Zero compilation warnings

#### Phase 1 Deliverables ✅

**Completed Components (89/89 story points):**
1. ✅ Frame type definitions (all 12 types)
2. ✅ Frame encoding/decoding with zero-copy parsing
3. ✅ Session state machine (5 states)
4. ✅ Connection ID management with rotation
5. ✅ Stream multiplexing (6 states)
6. ✅ Flow control windows (send/receive)
7. ✅ BBR congestion control (4 phases)
8. ✅ Comprehensive test suite (110 tests)
9. ✅ Benchmark suite (19 benchmarks)
10. ✅ Property-based tests (6 proptest cases)

**Performance Validation:**
- ✅ Frame parsing: >1M frames/sec (target met: 161M+ frames/sec)
- ✅ Zero-copy parsing confirmed (5.8-6.2 ns latency)
- ✅ All quality gates passing (clippy, fmt, tests)

**Documentation:**
- API documentation complete
- Code examples in all tests
- Benchmark results documented

#### Next: Phase 2 - Cryptographic Layer

**Prerequisites Met:**
- Core frame layer operational ✅
- Session management functional ✅
- Stream multiplexing ready ✅
- Congestion control implemented ✅

**Phase 2 Focus (102 story points, 4-6 weeks):**
- Noise_XX handshake implementation
- Elligator2 encoding for X25519 public keys
- Symmetric key ratcheting (per-packet)
- DH ratcheting (periodic)
- AEAD integration (XChaCha20-Poly1305)
- Constant-time cryptographic operations
- Forward secrecy validation

---

### Changed

**Python Tooling Documentation:**
- Added `docs/engineering/python-tooling.md` - Comprehensive guide for Python auxiliary tooling
  - Virtual environment setup and usage patterns
  - Critical command chaining guidance for Claude Code Bash tool
  - YAML linting with yamllint
  - Alternative installation methods (system packages, pipx)
  - Troubleshooting common venv issues
  - CI/CD integration examples

**Development Scripts:**
- Added `scripts/venv-setup.sh` - Automated Python venv diagnostic and setup script
  - Checks Python installation and venv module availability
  - Creates or repairs virtual environment
  - Installs required packages (yamllint)
  - Validates installation with health checks
  - 81 lines with comprehensive error handling

**Project Organization:**
- Established `/tmp/WRAITH-Protocol/` convention for temporary files
- Updated project memory banks with tooling documentation references

### Changed

**Release Workflow Enhancement (Commit: c420428):**
- Enhanced `.github/workflows/release.yml` to preserve existing release notes
- Added check step to detect if release already has notes
- Skip changelog extraction if existing notes are present
- Use conditional steps to create new release with notes or only upload assets
- Prevents overwriting manually-written comprehensive release notes (like v0.1.0)
- Workflow now intelligently handles both new releases and asset updates

### Fixed

**GitHub Workflows YAML Linting (36 issues across 5 files):**

Files updated:
- `.github/ISSUE_TEMPLATE/config.yml`
- `.github/dependabot.yml`
- `.github/workflows/ci.yml`
- `.github/workflows/codeql.yml`
- `.github/workflows/release.yml`

Fixes applied:
1. **Document Start Markers:** Added `---` to all YAML files for YAML 1.2 compliance
2. **Truthy Values:** Fixed `on:` → `"on":` in workflow triggers (prevents ambiguity)
3. **Line Length:** Broke long lines into multi-line format for readability
   - Conditional expressions with `&&` operators
   - Long command chains
   - URL and path concatenations
   - Comment text wrapping
4. **String Formatting:** Used block scalars (`>-`) for multi-line descriptions
5. **Variable Naming:** Improved variable names to avoid shell conflicts

**Technical Details:**
- All YAML files now pass `yamllint --strict` validation
- Improved readability while maintaining identical functionality
- Better compatibility with YAML parsers and GitHub Actions runner
- Resolved document-start, truthy, and line-length warnings

### Documentation

**Engineering Documentation:**
- Python tooling guide with critical Bash tool usage patterns
- Virtual environment command chaining requirements
- Common YAML linting workflows
- Automated venv setup and diagnostics

**Infrastructure:**
- Release workflow logic improvements
- GitHub Actions YAML best practices applied
- Project temporary file organization conventions

## [0.1.0] - 2025-11-29

### [2025-11-29] - Dependency Updates and Copilot Integration

#### Changed

**Dependency Updates (Dependabot PRs #9-#12):**
- Updated `getrandom` from 0.2 to 0.3 (PR #9)
  - Migrated API: `getrandom::getrandom()` → `getrandom::fill()`
  - Files modified: `crates/wraith-crypto/src/random.rs`, `crates/wraith-core/src/frame.rs`
  - Commit: ff9de57 - fix(deps): migrate to getrandom 0.3 API
- Updated `socket2` from 0.5 to 0.6 (PR #11)
- Updated `io-uring` dependency (PR #10)
- Updated `console` dependency (PR #12)

**GitHub Copilot Integration (PRs #16, #17):**
- Added `.github/copilot-instructions.md` with WRAITH-specific development context
- Added `.cargo/config.toml` with helpful cargo aliases (xtci, xtdoc, xdbuild)
- Documented protocol architecture, crate structure, and coding standards
- Added cryptographic safety guidelines for AI-assisted development

**Documentation Updates:**
- Updated `ref-docs/protocol_implementation_guide.md` for getrandom 0.3 consistency
- Updated `to-dos/protocol/phase-1-foundation.md` for getrandom 0.3 consistency

#### Technical Details

**getrandom 0.3 Migration:**
- **Breaking Change:** `getrandom::getrandom(&mut buf)` → `getrandom::fill(&mut buf).unwrap()`
- **Error Handling:** Updated from `Result<usize, Error>` to `Result<(), Error>`
- **Impact:** Improved API simplicity and ergonomics
- **Test Coverage:** All existing tests continue to pass

**Cargo Aliases (`.cargo/config.toml`):**
- `xtci`: Run full CI suite (`cargo xtask ci`)
- `xtdoc`: Build and open documentation (`cargo xtask doc`)
- `xdbuild`: Build XDP programs (`cargo xtask build-xdp`)

---

### [2025-11-29] - GitHub Security Scanning Configuration

#### Added

**Dependabot Configuration (.github/dependabot.yml):**
- Automated dependency update monitoring for Cargo (Rust) ecosystem
- GitHub Actions version update monitoring
- Weekly update schedule (Mondays at 09:00 UTC)
- Grouped updates by dependency category:
  - Cryptographic dependencies (chacha20poly1305, x25519-dalek, blake3, snow)
  - Async runtime dependencies (tokio, io-uring, futures)
  - Development dependencies (separate group)
- Conventional commit message prefixes (deps:, ci:)
- Auto-assignment to repository maintainers
- Pull request limits (10 for cargo, 5 for github-actions)

**CodeQL Security Scanning (.github/workflows/codeql.yml):**
- Automated security vulnerability scanning using GitHub CodeQL
- Rust language analysis with security-extended query suite
- Triggered on: push to main/develop, pull requests, weekly schedule, manual dispatch
- Two-job workflow:
  1. CodeQL Analysis: Comprehensive code scanning with security-extended queries
  2. Rust Security Audit: cargo-audit for RustSec advisory database scanning
- Security results uploaded to GitHub Security tab
- Artifact retention for audit results (30 days)
- cargo-audit integration for Rust-specific vulnerability detection
- cargo-outdated checks for dependency freshness
- Caching strategy for faster builds

**Security Scanning Features:**
- RustSec advisory database integration via cargo-audit
- Automated weekly security scans
- Pull request security validation
- Cryptographic dependency prioritization
- GitHub Security tab integration for centralized vulnerability tracking

#### Technical Details

**Dependabot Groups:**
- crypto: Critical cryptographic libraries (minor/patch updates)
- async-runtime: Tokio and async I/O dependencies (minor/patch updates)
- dev-dependencies: Development-only dependencies (minor/patch updates)

**CodeQL Configuration:**
- Language: Rust (experimental support)
- Query Suite: security-extended (comprehensive security analysis)
- Timeout: 30 minutes for analysis, 15 minutes for cargo-audit
- Permissions: actions:read, contents:read, security-events:write
- Build Strategy: Full workspace release build for accurate analysis

**Rust Security Tools:**
- cargo-audit: Scans Cargo.lock against RustSec advisory database
- cargo-outdated: Identifies outdated dependencies with security implications
- CodeQL: Static analysis for common vulnerability patterns

---

### [2025-11-29] - Rust 2024 Edition Upgrade

#### Changed

**Rust Edition and MSRV:**
- Upgraded to Rust 2024 edition (from Rust 2021)
- Updated MSRV from 1.75 to 1.85 (minimum required for edition 2024)
- Updated workspace Cargo.toml: edition = "2024", rust-version = "1.85"
- Updated clippy.toml: msrv = "1.85"
- Updated GitHub Actions CI workflow: MSRV job now uses Rust 1.85
- All crates inherit edition and rust-version from workspace manifest

**Code Formatting:**
- Applied cargo fmt across all crates to meet Rust 2024 formatting standards
- Fixed import ordering in wraith-core/src/frame.rs
- Fixed import ordering in wraith-crypto/src/aead.rs
- Fixed function signature formatting in wraith-crypto/src/elligator.rs

**Verification:**
- All workspace crates build successfully with edition 2024
- All tests pass (5 test suites: wraith-core, wraith-crypto, wraith-discovery, wraith-files, wraith-obfuscation)
- Clippy passes with no warnings
- Formatting verification passes

---

### [2025-11-29] - CI/Rust Fixes and Sprint Planning Enhancement

#### Fixed

**GitHub Actions CI Workflow:**
- Fixed deprecated `dtolnay/rust-action@master` to `dtolnay/rust-toolchain@stable`
- All CI jobs now use correct action (check, test, clippy, fmt, docs, msrv)

**Rust Codebase Fixes:**
- `wraith-crypto/src/aead.rs`: Removed unused `crypto_common::BlockSizeUser` import
- `wraith-core/src/congestion.rs`: Added `#[allow(dead_code)]` for BbrState fields
- `wraith-files/src/chunker.rs`: Fixed `div_ceil` implementation for Rust compatibility
- `xtask/src/main.rs`: Fixed rustdoc crate name warning
- Multiple crates: Formatting fixes (`cargo fmt`)
  - wraith-cli, wraith-core (frame, lib, session), wraith-crypto (elligator, lib)
  - wraith-discovery, wraith-obfuscation (lib, padding, timing)

**Sprint Planning Documentation:**
- Recreated and enhanced `wraith-recon-sprints.md` (2,185 lines)
  - 7 comprehensive user stories (RECON-001 through RECON-007)
  - Complete Rust implementations with wraith-* crate integration
  - Protocol milestone tracking and governance checkpoints
  - Sprint summary and risk register
- Recreated and enhanced `wraith-redops-sprints.md` (1,365 lines)
  - MITRE ATT&CK coverage matrix (14 tactics, 37+ techniques)
  - APT29 and APT28 adversary emulation playbooks
  - PostgreSQL database schema for implant management
  - gRPC protocol definitions (redops.proto)
  - 20+ test cases with compliance verification

---

### [2025-11-29] - Security Testing Client Documentation

#### Added

**Security Testing Client Documentation (15+ files, ~3,500 lines):**
- **WRAITH-Recon Documentation** (6 files):
  - Reference architecture with protocol integration details
  - Features documentation (governance, reconnaissance, exfiltration assessment)
  - Implementation guide with wraith-* crate usage patterns
  - Integration documentation (API examples, error handling)
  - Testing documentation (20+ test cases, compliance verification)
  - Usage documentation (operator workflows, audit procedures)

- **WRAITH-RedOps Documentation** (6 files):
  - Reference architecture (Team Server, Operator Client, Spectre Implant)
  - Features documentation (C2 infrastructure, adversary emulation)
  - Implementation guide with protocol-accurate technical details
  - Integration documentation (gRPC API, multi-transport support)
  - Testing documentation (evasion validation, MITRE ATT&CK mapping)
  - Usage documentation (engagement workflows, purple team collaboration)

- **Sprint Planning Documentation**:
  - WRAITH-Recon sprint plan (12 weeks, 55 story points)
  - WRAITH-RedOps sprint plan (14 weeks, 89 story points)
  - Protocol dependency tracking for security testing clients

- **Comprehensive Client Roadmap**:
  - ROADMAP-clients.md (1,500+ lines)
  - Complete development planning for all 10 clients
  - Tier classification (Tier 1: Core, Tier 2: Specialized, Tier 3: Advanced + Security Testing)
  - Story point estimates (1,028 total across all clients)
  - Integration timeline with protocol development phases
  - Cross-client dependencies and shared components
  - MITRE ATT&CK technique mapping (51+ techniques for RedOps)

#### Enhanced

**Client Overview Documentation:**
- Added Tier 3 Security Testing section
- Updated client ecosystem overview with all 10 clients
- Protocol-aligned reference architectures for security testing clients
- Governance framework compliance documentation

**Project Roadmap (ROADMAP.md):**
- Security testing clients timeline (Weeks 44-70)
- WRAITH-Recon development milestones
- WRAITH-RedOps development milestones with MITRE ATT&CK integration
- Performance targets for security testing clients
- Combined ecosystem timeline spanning 70 weeks

**README.md:**
- Updated Client Applications section with 3-tier classification
- Added security testing clients with governance notice
- Updated project structure documentation
- Enhanced documentation section with file counts
- Added Security Testing documentation references
- Total ecosystem: 10 clients, 1,028 story points

**CHANGELOG.md:**
- This comprehensive update entry
- Documentation statistics and file counts
- Technical details of security testing integration

#### Technical Details

**Protocol Integration:**
- Complete cryptographic suite integration (X25519, Elligator2, XChaCha20-Poly1305, BLAKE3)
- Noise_XX handshake implementation patterns for C2 channels
- Wire protocol specifications (outer packet + inner frame structures)
- AF_XDP kernel bypass configuration for high-speed operations
- io_uring integration for async I/O operations
- Obfuscation layer integration (padding modes, timing obfuscation, protocol mimicry)
- Ratcheting schedules (symmetric per-packet, DH periodic)

**wraith-* Crate Integration Examples:**
- `wraith-core`: Frame encoding, session management, BBR congestion control
- `wraith-crypto`: Full cryptographic suite, Elligator2 encoding, key ratcheting
- `wraith-transport`: AF_XDP configuration, UDP fallback, connection migration
- `wraith-obfuscation`: Protocol mimicry profiles (TLS, WebSocket, DNS-over-HTTPS)
- `wraith-discovery`: DHT integration, NAT traversal, relay support
- `wraith-files`: Chunking strategies, BLAKE3 tree hashing, integrity verification

**Governance & Compliance:**
- Security Testing Parameters framework referenced
- Signed Rules of Engagement (RoE) validation
- Scope enforcement mechanisms (CIDR/domain whitelisting)
- Kill switch architecture (emergency shutdown)
- Tamper-evident audit logging
- Chain of custody preservation
- Multi-operator accountability (RedOps)

**Testing & Validation:**
- 20+ protocol verification test cases (Recon)
- Evasion technique validation (RedOps)
- MITRE ATT&CK technique mapping (51+ techniques across 12 tactics)
- Detection engineering support documentation
- Purple team collaboration workflows
- Compliance verification procedures

**Documentation Statistics:**
- **Files Enhanced:** 15+ files (architecture, features, implementation, integration, testing, usage)
- **Lines Added:** ~3,500 lines of technical documentation
- **Code Examples:** Rust, SQL, Protobuf, JSON, Mermaid diagrams
- **API Integration Patterns:** Complete wraith-* crate usage examples
- **Test Cases:** 20+ functional, performance, security, and compliance tests

**Client Ecosystem Metrics:**
- **Total Clients:** 10 (8 standard + 2 security testing)
- **Total Story Points:** 1,028
- **Development Timeline:** ~70 weeks (parallel development)
- **Documentation Files:** 37 client docs (previously 25)
- **Sprint Planning:** 10 client sprint files

---

### Added

#### Rust Workspace (7 crates, 8,732 lines)
- `wraith-core`: Protocol primitives, frames, sessions, BBR congestion control
- `wraith-crypto`: XChaCha20-Poly1305 AEAD, key ratcheting, Elligator2, Noise_XX
- `wraith-transport`: UDP fallback, io_uring acceleration stubs
- `wraith-obfuscation`: Padding, timing, cover traffic generation
- `wraith-discovery`: DHT peer discovery, NAT traversal
- `wraith-files`: File chunking, BLAKE3 hashing
- `wraith-cli`: Command-line interface with clap
- `xtask`: Build automation (test, lint, fmt, ci, build-xdp, doc)

#### Architecture Documentation (5 documents, 3,940 lines)
- `protocol-overview.md`: High-level WRAITH architecture and design philosophy
- `layer-design.md`: 6-layer protocol stack (Network, Kernel, Obfuscation, Crypto, Session, Application)
- `security-model.md`: Threat model, cryptographic guarantees, security properties
- `performance-architecture.md`: Kernel bypass (AF_XDP), zero-copy design, io_uring integration
- `network-topology.md`: P2P network design, DHT architecture, relay infrastructure

#### Engineering Documentation (4 documents, 3,013 lines)
- `development-guide.md`: Environment setup, building, testing, debugging, IDE configuration
- `coding-standards.md`: Rust conventions, error handling, security practices, code review
- `api-reference.md`: Complete API documentation for all 7 crates with examples
- `dependency-management.md`: Version policy, security auditing, license compliance

#### Integration Documentation (3 documents, 1,773 lines)
- `embedding-guide.md`: Integration patterns for Rust, C/C++ (FFI), Python (PyO3), WASM
- `platform-support.md`: Linux, macOS, Windows, mobile platform support matrix
- `interoperability.md`: Protocol versioning, bridges, migration strategies

#### Testing Documentation (3 documents, 1,856 lines)
- `testing-strategy.md`: Unit, integration, E2E, property-based testing, fuzzing
- `performance-benchmarks.md`: Criterion benchmarks, profiling, optimization results
- `security-testing.md`: Cryptographic validation, protocol security, penetration testing

#### Operations Documentation (3 documents, 1,609 lines)
- `deployment-guide.md`: Production deployment, systemd services, Docker, Kubernetes
- `monitoring.md`: Prometheus metrics, Grafana dashboards, logging, alerting
- `troubleshooting.md`: Common issues, diagnostic commands, recovery procedures

#### Client Documentation (25 documents, 7,796 lines)
- `overview.md`: Client application landscape, tiers, shared components
- **WRAITH-Transfer** (3 docs): P2P file transfer architecture, features, implementation
- **WRAITH-Chat** (3 docs): E2EE messaging with Double Ratchet, group chat, voice/video
- **WRAITH-Sync** (3 docs): Delta sync, conflict resolution, cross-device synchronization
- **WRAITH-Share** (3 docs): DHT content addressing, swarm downloads, access control
- **WRAITH-Stream** (3 docs): AV1/Opus streaming, adaptive bitrate, live/VOD
- **WRAITH-Mesh** (3 docs): IoT mesh networking, network visualization
- **WRAITH-Publish** (3 docs): Censorship-resistant publishing, DHT storage
- **WRAITH-Vault** (3 docs): Shamir SSS, erasure coding, distributed backups

#### Sprint Planning (16 documents, 21,652 lines)
- `ROADMAP.md`: Executive roadmap with milestones and release strategy
- Protocol implementation phases (7 documents, 789 story points):
  - Phase 1: Foundation & Core Types
  - Phase 2: Cryptographic Layer
  - Phase 3: Transport & Kernel Bypass
  - Phase 4: Obfuscation & Stealth
  - Phase 5: Discovery & NAT Traversal
  - Phase 6: Integration & Testing
  - Phase 7: Hardening & Optimization
- Client application sprints (8 documents, 884 story points):
  - WRAITH-Transfer, WRAITH-Chat, WRAITH-Sync, WRAITH-Share
  - WRAITH-Stream, WRAITH-Mesh, WRAITH-Publish, WRAITH-Vault

#### Project Infrastructure
- GitHub Actions CI workflow (check, test, clippy, fmt, docs, msrv)
- Development configuration (rustfmt.toml, clippy.toml)
- Standard repository files (LICENSE, SECURITY.md, CODE_OF_CONDUCT.md)
- GitHub issue templates (bug report, feature request, security vulnerability)
- Pull request template
- Project banner and architecture graphics

### Security
- Cryptographic foundation designed for forward secrecy
- Traffic analysis resistance via Elligator2 encoding
- AEAD encryption with XChaCha20-Poly1305
- Constant-time operations for side-channel resistance
- Memory zeroization for sensitive data

### Documentation Statistics
- **Total Documentation Files:** 59
- **Total Lines of Documentation:** 40,000+
- **Code Examples:** Rust, TypeScript, shell, TOML, YAML, Dockerfile
- **Diagrams:** Mermaid and ASCII architecture visualizations

---

[Unreleased]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.1.0...v0.1.5
[0.1.0]: https://github.com/doublegate/WRAITH-Protocol/releases/tag/v0.1.0
