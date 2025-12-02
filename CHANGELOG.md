# Changelog

All notable changes to WRAITH Protocol will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.8.0] - 2025-12-01

### Added
- **7 Integration Tests (19 SP)**: Component integration testing
  - End-to-end file transfer (5MB), resume with missing chunks
  - Multi-peer coordination (3 peers, 20 chunks), NAT traversal components
  - Relay fallback, obfuscation modes, Noise_XX + ratcheting
- **Security Audit Template (DOC-004, 4 SP)**: Comprehensive review checklist
  - 10 sections: crypto, memory, side-channels, network, dependencies, etc.
  - Penetration testing scope, fuzzing/sanitizer commands
- **Private Key Encryption (SEC-001, 13 SP)**: Argon2id + XChaCha20-Poly1305
  - `encrypted_keys.rs` module (705 LOC, 16 tests)
  - Argon2id key derivation with configurable parameters (OWASP-recommended defaults)
  - XChaCha20-Poly1305 AEAD encryption for private keys
  - `EncryptedPrivateKey` struct with binary serialization
  - `DecryptedPrivateKey` wrapper with `ZeroizeOnDrop`
  - Passphrase rotation via `change_passphrase()`
  - Security presets: `low_security()`, `default()`, `high_security()`

### Changed
- **BLAKE3 SIMD (PERF-001, 8 SP)**: rayon + neon features
  - 2-4x faster parallel hashing, ARM64 optimization

### Refactored
- **AEAD Module Split (REFACTOR-001, 8 SP)**: Improved code organization
  - Split 1,529 LOC `aead.rs` into 4 focused modules (1,251 LOC total)
  - `aead/cipher.rs` (488 LOC) - Nonce, Tag, AeadKey, AeadCipher
  - `aead/replay.rs` (264 LOC) - ReplayProtection sliding window
  - `aead/session.rs` (457 LOC) - SessionCrypto, BufferPool
  - `aead/mod.rs` (42 LOC) - Re-exports for backward compatibility
  - All 23 AEAD tests preserved and passing

### Documentation
- Refactoring analysis (18 priorities, complexity metrics)

**Story Points: 52 SP**

## [0.7.0] - 2025-12-01

### Added

**Phase 7: Hardening & Optimization - COMPLETE (2025-12-01):**

This release completes Phase 7, delivering security hardening, fuzzing infrastructure, performance optimization, comprehensive documentation, and cross-platform packaging for production readiness.

#### Sprint 7.1: Security Audit (34 SP) - COMPLETE

**Security Review and Hardening:**
- Comprehensive security audit of all cryptographic implementations
- Code review checklist for constant-time operations
- Verification of ZeroizeOnDrop on all secret key material
- Review of unsafe code blocks with SAFETY comments
- Threat modeling documentation updates
- Security best practices documentation

#### Sprint 7.2: Fuzzing & Property Testing (26 SP) - COMPLETE

**Fuzzing Infrastructure (fuzz/ - NEW):**
- `fuzz/Cargo.toml` - libfuzzer-sys configuration
- 5 fuzz targets for critical parsing paths:
  - `frame_parser.rs` - Frame parsing with arbitrary bytes
    - Tests both SIMD and scalar parsing paths
    - Ensures parser never panics on malformed input
  - `dht_message.rs` - DHT message parsing
    - Validates Kademlia message handling
    - Tests FIND_NODE, FIND_VALUE, STORE operations
  - `padding.rs` - Padding engine with all modes
    - Tests PowerOfTwo, SizeClasses, ConstantRate, Statistical
    - Validates padding/depadding round-trips
  - `crypto.rs` - Cryptographic primitives
    - AEAD encrypt/decrypt fuzzing
    - Key derivation input validation
  - `tree_hash.rs` - BLAKE3 tree hashing
    - Merkle tree construction with arbitrary data
    - Incremental hasher state transitions

**Property-Based Testing:**
- proptest integration for frame validation
- Invariant testing for state machines
- Round-trip property tests for serialization

#### Sprint 7.3: Performance Optimization (47 SP) - COMPLETE

**O(m) Missing Chunks Optimization (wraith-files/src/chunker.rs):**
- Dual-tracking pattern with `missing_chunks: HashSet` and `received_chunks: HashSet`
- `missing_chunks()` returns iterator over missing set - O(m) where m = missing count
- `missing_count()` returns missing set length - O(1)
- `is_chunk_missing()` uses HashSet lookup - O(1)
- `has_chunk()` uses HashSet lookup - O(1)
- Previous O(n) iteration replaced with O(1)/O(m) operations
- Critical for large file resume operations (10,000+ chunks)

**Allocation-Free Hashing (wraith-files/src/tree_hash.rs):**
- `IncrementalTreeHasher::update()` uses slice references
- No intermediate Vec allocations during hash computation
- Zero-copy chunk boundary detection
- Pre-allocated leaf hash vector in `finalize()`
- Memory-efficient streaming for multi-gigabyte files

**Performance Benchmarks (crates/wraith-files/benches/files_bench.rs - NEW 400 lines):**
- FileReassembler benchmarks:
  - `bench_missing_chunks_by_completion` - Validates O(m) scaling at 0%, 50%, 90%, 95%, 99%, 100%
  - `bench_missing_count` - Validates O(1) count operation
  - `bench_is_chunk_missing` - Validates O(1) membership test
  - `bench_chunk_write` - Sequential and random write patterns
- IncrementalTreeHasher benchmarks:
  - `bench_incremental_hasher_update` - Update throughput (1KB-64KB)
  - `bench_incremental_hasher_full` - End-to-end streaming (1MB-100MB)
- Merkle tree benchmarks:
  - `bench_merkle_root_computation` - Root calculation (4-4096 leaves)
  - `bench_tree_hash_from_data` - Full file hashing (1MB-100MB)
- FileChunker benchmarks:
  - `bench_file_chunking` - Sequential read throughput (1MB-100MB)
  - `bench_random_access_chunking` - Seek and read performance

**Profiling Infrastructure (scripts/profile.sh - NEW 234 lines):**
- CPU profiling with perf and flamegraph
  - `profile_cpu()` - Generates SVG flamegraphs for hotspot analysis
  - Targets transfer and crypto benchmarks
- Memory profiling with valgrind
  - `profile_memory()` - Uses massif for allocation tracking
  - Leak detection with full stack traces
- Cache profiling with perf stat
  - `profile_cache()` - L1/L2 cache hit rates
  - Instructions per cycle analysis
- Benchmark runner
  - `run_benchmarks()` - Full criterion suite with HTML reports
- Usage: `./scripts/profile.sh [cpu|memory|cache|bench|all]`

**Benchmark Results:**
- Missing chunks (99% complete, 10K total): <1us (was O(n), now O(m))
- Missing count: <100ns regardless of file size
- Tree hashing: >3 GiB/s (in-memory)
- Merkle root (4096 leaves): <50us
- File chunking: >1.5 GiB/s
- Chunk verification: <1us per 256 KiB chunk

#### Sprint 7.4: Documentation (26 SP) - COMPLETE

**User Documentation (docs/USER_GUIDE.md - NEW ~800 lines):**
- Installation guide (pre-built binaries, build from source)
- Quick start tutorial with examples
- CLI command reference (send, receive, daemon, status, peers, keygen)
- Configuration guide with all sections explained
- File transfer workflows (single file, multi-peer, resume)
- Obfuscation modes (none, low, medium, high, paranoid)
- Multi-peer download coordination
- Troubleshooting guide with common issues
- FAQ section
- Security best practices

**Configuration Reference (docs/CONFIG_REFERENCE.md - NEW ~650 lines):**
- Complete TOML configuration reference
- All configuration sections documented:
  - `[node]` - Node identity and keypair
  - `[network]` - Listen address, ports, connections
  - `[transport]` - AF_XDP, io_uring, UDP settings
  - `[session]` - Timeouts, retransmission, SACK
  - `[congestion]` - BBR parameters
  - `[obfuscation]` - Padding, timing, mimicry
  - `[discovery]` - DHT, relay, NAT traversal
  - `[transfer]` - Chunking, multi-peer, resume
  - `[files]` - io_uring, direct I/O settings
  - `[logging]` - Levels, formats, audit
  - `[security]` - Replay protection, ratcheting
  - `[metrics]` - Prometheus endpoint
- Environment variable mappings
- Example configurations:
  - Minimal configuration
  - High-performance server
  - Privacy-focused configuration
  - Relay server configuration

**API Reference Updates (docs/engineering/api-reference.md):**
- TransferSession documentation (methods, states, multi-peer)
- FileChunker documentation (sequential/random access)
- FileReassembler documentation (O(m) optimization explained)
- FileTreeHash and tree_hash functions documentation
- IncrementalTreeHasher documentation

**Deployment Guide Updates (docs/operations/deployment-guide.md):**
- Expanded Performance Tuning section:
  - System tuning (sysctl, ulimits)
  - CPU optimization (isolcpus, NUMA)
  - AF_XDP performance tuning
  - io_uring optimization
  - Benchmark expectations
- Comprehensive Security Hardening section:
  - File system permissions
  - User/group configuration
  - Systemd security directives
  - SELinux policy module
  - AppArmor profile
  - Network security (iptables/nftables)
  - Security audit checklist
  - Security monitoring

#### Sprint 7.5: Cross-Platform & Packaging (25 SP) - COMPLETE

**Cross-Platform CI Testing (.github/workflows/ci.yml):**
- Added test matrix for Linux, macOS, and Windows
- Platform-specific test flags (Windows uses limited threads)
- Enhanced caching strategy per platform
- Documentation header with job descriptions

**Packaging Script (scripts/package.sh - NEW 400 lines):**
- Multi-format package generation:
  - `tar.gz` - Generic Linux tarball with docs and example config
  - `deb` - Debian/Ubuntu package with systemd service
  - `rpm` - Fedora/RHEL package with systemd service
- Package features:
  - Automatic version extraction from Cargo.toml
  - Architecture detection (x86_64, aarch64)
  - SHA256 checksum generation
  - Stripped binaries for smaller size
  - Example configuration files
  - Systemd service with security hardening
  - Pre/post install scripts for user/group creation
- Usage: `./scripts/package.sh [deb|rpm|tar|all]`

**Package Contents:**
- Binary: `/usr/bin/wraith`
- Config: `/etc/wraith/config.toml.example`
- Service: `/lib/systemd/system/wraith.service`
- Docs: README.md, LICENSE, CHANGELOG.md, USER_GUIDE.md

**Systemd Service Features:**
- Automatic user/group creation (wraith:wraith)
- Security hardening (NoNewPrivileges, ProtectSystem, etc.)
- Resource limits (NOFILE=65536, NPROC=4096)
- Automatic restart on failure

### Changed

- **Version:** 0.6.0 -> 0.7.0
- **Documentation Structure:**
  - Added USER_GUIDE.md for end-user documentation
  - Added CONFIG_REFERENCE.md for configuration documentation
  - Expanded api-reference.md with Phase 6 components
  - Expanded deployment-guide.md with security/performance sections
- **FileReassembler Performance:**
  - `missing_chunks()` changed from O(n) iteration to O(m) HashSet return
  - Added `missing_count()` for O(1) count queries
  - Dual HashSet tracking for optimal performance

### Fixed

- **Performance Issues:**
  - O(n) missing chunks iteration replaced with O(m) HashSet operations
  - Allocation overhead in incremental tree hashing eliminated
  - Memory efficiency improved for large file transfers

### Phase 7 Complete

**All Sprints Completed:**
- Sprint 7.1: Security Audit (34/34 SP) - COMPLETE
- Sprint 7.2: Fuzzing & Property Testing (26/26 SP) - COMPLETE
- Sprint 7.3: Performance Optimization (47/47 SP) - COMPLETE
- Sprint 7.4: Documentation (26/26 SP) - COMPLETE
- Sprint 7.5: Cross-Platform & Packaging (25/25 SP) - COMPLETE

**Phase 7 Progress:** 158/158 SP complete (100%)

---

## [0.6.0] - 2025-11-30

### Added

**Phase 6: Integration & End-to-End Testing - COMPLETE ✅ (2025-11-30):**

This release completes Phase 6, integrating all protocol components into a cohesive file transfer engine with comprehensive CLI implementation and performance validation.

#### Sprint 6.1: File Chunking & Hashing (21 SP)

**Enhanced File Chunking (wraith-files/src/chunker.rs):**
- Complete `FileChunker` implementation with file I/O and seek support
  - Configurable chunk sizes (default: 256 KiB)
  - File size and chunk count tracking
  - Sequential chunk reading with automatic offset management
  - Seek support for random access to specific chunks
  - Total chunks calculation with proper ceiling division
  - 4 comprehensive tests
- Complete `FileReassembler` implementation for out-of-order chunk reception
  - Pre-allocated file creation with target size
  - Out-of-order chunk writing with offset calculation
  - Received chunks tracking via HashSet
  - Missing chunks detection for resume support
  - Completion status checking
  - 2 comprehensive tests

**BLAKE3 Tree Hashing (wraith-files/src/tree_hash.rs - NEW 320 lines):**
- `FileTreeHash` structure for Merkle tree representation
  - Root hash (32 bytes) for complete file verification
  - Per-chunk hashes for individual chunk verification
- `compute_tree_hash()` for file-based tree hashing
  - Reads file in chunks and computes BLAKE3 hash for each
  - Builds Merkle tree from leaf hashes
  - Returns root hash and all chunk hashes
- `compute_merkle_root()` for Merkle tree construction
  - Binary tree construction from leaf hashes
  - Recursive hashing of paired nodes
  - Single-node handling for odd number of leaves
- `verify_chunk()` for chunk integrity verification
  - Validates chunk data against stored chunk hash
  - Constant-time comparison for security
- `IncrementalTreeHasher` for streaming hash computation
  - Buffered chunk accumulation
  - Automatic chunk boundary detection
  - Finalization with partial chunk handling
  - 11 comprehensive tests including incremental hashing

**Performance:**
- Tree hashing throughput: >3 GiB/s (in-memory)
- Chunk verification: <1μs per chunk

#### Sprint 6.2: Transfer State Machine (26 SP)

**Transfer Session Management (wraith-core/src/transfer/session.rs - NEW 615 lines):**
- `TransferSession` state machine with progress tracking
  - Transfer ID generation (32-byte unique identifier)
  - Direction tracking (Send/Receive)
  - File path and size management
  - Configurable chunk size (default: 256 KiB)
  - Total chunks calculation
- 7-state transfer lifecycle:
  - Initializing: Setup phase
  - Handshaking: Peer negotiation
  - Transferring: Active data transfer
  - Paused: Temporary suspension
  - Completing: Finalization phase
  - Complete: Successfully finished
  - Failed: Error termination
- Progress tracking and metrics:
  - Transferred chunks set (HashSet for O(1) lookup)
  - Bytes transferred counter
  - Start time tracking
  - Last activity timestamp
  - Transfer speed calculation (bytes/sec)
  - ETA estimation (remaining bytes / speed)
  - Completion percentage
- Multi-peer download coordination:
  - Per-peer state tracking (active, bytes transferred, last activity)
  - Chunk assignment across peers
  - Load balancing for parallel downloads
  - Peer health monitoring
  - Automatic peer removal on timeout
- Pause/resume support:
  - State persistence for resume
  - Missing chunks calculation
  - Progress restoration
- 9 comprehensive tests

**BBR Congestion Control Integration:**
- Already implemented in Phase 4 (wraith-core/src/congestion.rs, 1412 lines)
- No additional work required for Sprint 6.2.2

#### Sprint 6.3: CLI Implementation (26 SP)

**Configuration System (wraith-cli/src/config.rs - NEW 370 lines):**
- TOML-based configuration with serde
- Configuration structure:
  - `NodeConfig`: Node ID, keypair paths, data directory
  - `NetworkConfig`: Listen address, ports, max peers
  - `ObfuscationConfig`: Padding mode, timing mode, protocol mimicry
  - `DiscoveryConfig`: Bootstrap nodes, DHT enabled, relay mode
  - `TransferConfig`: Chunk size, max concurrent transfers, resume enabled
  - `LoggingConfig`: Level, file path, console output
- `Config::load()` - Load from TOML file
- `Config::save()` - Save to TOML file
- `Config::load_or_default()` - Load or create default config
- `Config::validate()` - Comprehensive validation with detailed error messages
- Default configuration path: `~/.config/wraith/config.toml`

**Progress Display (wraith-cli/src/progress.rs - NEW 140 lines):**
- `TransferProgress` wrapper around indicatif ProgressBar
- Progress bar features:
  - Transfer speed (bytes/sec formatted as B/s, KiB/s, MiB/s, GiB/s)
  - ETA calculation and display
  - Bytes transferred / total bytes
  - Completion percentage
  - Chunk progress (chunks received / total chunks)
- Helper functions:
  - `format_bytes()` - Human-readable byte counts (B, KiB, MiB, GiB, TiB)
  - `format_speed()` - Human-readable transfer speeds
  - `format_duration()` - Human-readable time (s, m, h, d)

**CLI Commands (wraith-cli/src/main.rs - Enhanced 520 lines):**
- `send` - Send file to recipient
  - File path and recipient node ID arguments
  - Obfuscation mode selection (none, low, medium, high, paranoid)
  - Multi-peer download support
  - Progress bar with real-time updates
  - Completion notification
- `receive` - Receive files from peers
  - Output directory specification
  - Listen address configuration
  - Automatic file saving
  - Progress display for multiple transfers
- `daemon` - Run as background daemon
  - Persistent listen mode
  - Optional relay server mode
  - Signal handling for graceful shutdown
  - Logging to file
- `status` - Show node status and active transfers
  - Node ID and listening address
  - Active transfers with progress
  - Peer connections
  - Relay status
- `peers` - List discovered peers
  - Peer ID and last seen time
  - Connection type (direct, hole-punched, relayed)
  - Distance metric (DHT XOR distance)
- `keygen` - Generate new keypair
  - Ed25519 signing keypair
  - X25519 encryption keypair
  - PEM format output
  - Optional custom output path

#### Sprint 6.4: Integration & Performance Testing (25 SP)

**Integration Tests (tests/integration_tests.rs - Enhanced 470 lines):**
- 4 active integration tests:
  - `test_file_chunking_and_reassembly` - Complete chunking workflow
    - Create test file, chunk it, reassemble, verify integrity
  - `test_tree_hash_verification` - BLAKE3 tree hashing validation
    - Compute tree hash, verify individual chunks, tamper detection
  - `test_transfer_progress_tracking` - Transfer session state machine
    - Initialize transfer, mark chunks complete, track progress/speed/ETA
  - `test_multi_peer_coordination` - Multi-peer download
    - Add multiple peers, assign chunks, track per-peer progress
- 7 placeholder tests for Phase 7 (end-to-end protocol integration):
  - `test_end_to_end_transfer` - Complete file transfer workflow
  - `test_connection_establishment` - Handshake and session setup
  - `test_obfuscation_integration` - Padding and protocol mimicry
  - `test_discovery_integration` - DHT peer discovery
  - `test_multi_path_transfer` - Connection migration
  - `test_error_recovery` - Network failures and retransmission
  - `test_concurrent_transfers` - Multiple simultaneous transfers

**Performance Benchmarks (benches/transfer.rs - NEW 220 lines):**
- 5 active Criterion benchmarks:
  - `bench_file_chunking` - File read and chunking (1MB, 10MB, 100MB)
  - `bench_tree_hashing` - File-based BLAKE3 tree hashing (1MB, 10MB, 100MB)
  - `bench_tree_hashing_memory` - In-memory tree hashing (1MB, 10MB, 100MB)
  - `bench_chunk_verification` - Individual chunk validation
  - `bench_file_reassembly` - Out-of-order chunk writing (1MB, 10MB)
- 4 placeholder benchmarks for Phase 7:
  - `bench_transfer_throughput` - Full protocol throughput (target: >300 Mbps on LAN)
  - `bench_transfer_latency` - RTT and chunk delivery time (target: <10ms RTT on LAN)
  - `bench_bbr_utilization` - BBR bandwidth utilization (target: >95% link utilization)
  - `bench_multi_peer_speedup` - Multi-peer download speedup (target: linear to 5 peers)

**Benchmark Results:**
- File chunking: ~1.5 GiB/s (1MB file), ~2.0 GiB/s (100MB file)
- Tree hashing (file): ~2.5 GiB/s
- Tree hashing (memory): >3 GiB/s
- Chunk verification: <1μs per 256 KiB chunk
- File reassembly: ~800 MiB/s (1MB), ~1.2 GiB/s (10MB)

### Changed

- **Dependencies Added:**
  - wraith-cli: `toml = "0.8"`, `serde = { version = "1", features = ["derive"] }`, `dirs = "5.0"`, `hex = "0.4"`
  - tests: `wraith-files = { path = "../crates/wraith-files" }`, `tempfile = "3"`
- **Module Exports Updated:**
  - wraith-files: Added `tree_hash` module to public exports
  - wraith-core: Added `transfer` module to public exports
- **Test Configuration:**
  - Added `[[bench]]` section for transfer benchmarks in tests/Cargo.toml
- **Phase 7 Planning (Post-Phase 6):**
  - Added Section 7.3.4 End-to-End Benchmarks to phase-7-hardening.md (13 SP)
  - Documented 4 benchmark functions for Phase 7 implementation
  - Updated Phase 7 story points: 145 SP → 158 SP

### Fixed

- **Code Quality:**
  - All inner doc comments converted to regular comments in integration_tests.rs
  - Proper error handling in all CLI commands
  - Consistent use of `anyhow::Result` for error propagation
- **Clippy Warnings Resolved (Post-Phase 6):**
  - **dead_code:** Removed 4 unused Phase 7 placeholder benchmark functions (benches/transfer.rs)
    - Converted `bench_transfer_throughput`, `bench_transfer_latency`, `bench_bbr_utilization`, `bench_multi_peer_speedup` to comments
    - Functions preserved in phase-7-hardening.md (Section 7.3.4) for Phase 7 implementation
  - **manual_abs_diff:** Replaced manual absolute difference with `Duration::abs_diff()` (crates/wraith-core/src/congestion.rs)
    - Changed `if min_rtt > new_rtt { min_rtt - new_rtt } else { new_rtt - min_rtt }` to `min_rtt.abs_diff(new_rtt)`
  - **manual_range_contains:** Replaced 3 manual range checks with `RangeInclusive::contains()` (crates/wraith-obfuscation/src/timing.rs)
    - Changed `x >= min && x <= max` to `(min..=max).contains(&x)` for cleaner range validation
  - **empty_docs:** Fixed empty doc comment (tests/integration_tests.rs)
    - Changed `//!` to `//` for non-documentation comment

### Phase 6 Deliverables ✅

**Completed Components (98/98 story points):**
1. ✅ Enhanced file chunking with seek support and chunk indexing
2. ✅ BLAKE3 tree hashing with Merkle verification
3. ✅ Transfer session state machine with progress tracking
4. ✅ Multi-peer download coordination with chunk assignment
5. ✅ BBR congestion control integration (already complete)
6. ✅ Full CLI implementation (6 commands: send, receive, daemon, status, peers, keygen)
7. ✅ TOML configuration system with validation
8. ✅ Progress display with transfer speed and ETA
9. ✅ Comprehensive integration tests (4 active + 7 Phase 7 placeholders)
10. ✅ Performance benchmarks (5 active + 4 Phase 7 placeholders)

**Quality Gates:**
- ✅ All 911 tests passing (18 ignored for Phase 7)
- ✅ Clippy clean (zero warnings)
- ✅ rustfmt compliant
- ✅ Documentation builds successfully

**Performance Validation:**
- ✅ Tree hashing: >3 GiB/s
- ✅ File chunking: >1.5 GiB/s
- ✅ Chunk verification: <1μs per chunk
- ✅ File reassembly: >800 MiB/s

**Documentation:**
- Updated Phase 6 TODO to 100% complete
- Updated README with Phase 6 completion status
- Updated CLAUDE.local.md with new modules

**Next: Phase 7 - Hardening & Optimization**

**Prerequisites Met:**
- File transfer engine operational ✅
- CLI fully implemented ✅
- Integration tests ready ✅
- Performance benchmarks baseline ✅

**Phase 7 Focus (145 story points, 5-6 weeks):**
- End-to-end protocol integration
- Full file transfer workflow (handshake → transfer → verification)
- Obfuscation layer integration (padding, timing, protocol mimicry)
- Discovery integration (DHT, NAT traversal, relay)
- Security hardening (fuzzing, audit, penetration testing)
- Performance optimization (>300 Mbps throughput, <10ms RTT)

---

## [0.5.5] - 2025-11-30

### Security

- **SEC-001:** Implemented S/Kademlia crypto puzzle Sybil resistance
  - 20-bit difficulty requiring ~1M hash attempts for NodeId generation
  - O(1) verification, O(2^difficulty) generation
  - Protects DHT from Sybil and Eclipse attacks
- **SEC-002:** Implemented DHT privacy enhancement with group_secret
  - `info_hash = BLAKE3-keyed(group_secret, content_hash)`
  - Real file hashes never exposed in DHT lookups
  - Only participants with group_secret can derive lookup keys
- **SEC-003:** Implemented STUN MESSAGE-INTEGRITY authentication
  - RFC 5389 compliant HMAC-SHA1 authentication
  - Transaction ID validation
  - CRC-32 fingerprint verification
  - Rate limiting (10 req/s per IP default)

### Added

- `SybilResistance` struct for configurable crypto puzzle difficulty
- `GroupSecret` type with automatic zeroization
- `DhtPrivacy` module for privacy-preserving operations
- `StunAuthentication` struct for RFC 5389 auth
- `StunRateLimiter` for DoS protection
- 28 new security-focused tests

### Dependencies

- Added `hmac` 0.12 for HMAC-SHA1
- Added `sha1` 0.10 for SHA-1 hashing
- Added `md-5` 0.10 for long-term credential derivation

### Documentation

- Created `phase-5-tech-debt.md`
- Updated technical debt tracking documents
- Updated README with security features

## [0.5.0] - 2025-11-30

### Added

**Phase 5 Sprint 5.5: Integration & Testing - PHASE 5 COMPLETE (2025-11-30):**
- Implemented unified `DiscoveryManager` for seamless peer discovery
  - Orchestrates DHT, NAT traversal, and relay infrastructure
  - End-to-end connection flow: DHT lookup → direct → hole punch → relay fallback
  - Configuration system with `DiscoveryConfig` and `RelayInfo`
  - State management (`DiscoveryState`: Stopped, Starting, Running, Stopping)
  - Connection type tracking (`ConnectionType`: Direct, HolePunched, Relayed)
  - DHT bootstrap with configurable bootstrap nodes
  - Relay server connection and automatic registration
  - NAT type detection integration with STUN
  - 8 public methods + 6 helper methods
- Added comprehensive integration tests (15 tests)
  - Discovery manager lifecycle (creation, start, shutdown)
  - Configuration with bootstrap nodes, STUN servers, relay servers
  - NAT detection enable/disable scenarios
  - Relay enable/disable scenarios
  - Connection type variants and display
  - Peer discovery flow (DHT lookup, connection attempts)
  - Error handling and fallback behavior
  - Concurrent peer discovery
  - State transitions
- **Test Results:** 15 integration tests, all passing
- **Quality Gates:** All passing (cargo test, clippy, fmt)
- **Phase 5 Status:** ✅ **COMPLETE** (123/123 SP, 100%)
- **Components Delivered:**
  - Privacy-enhanced Kademlia DHT (Sprints 5.1-5.2)
  - DERP-style relay infrastructure (Sprints 5.3-5.4)
  - NAT traversal with STUN/ICE (Sprint 5.4)
  - Unified discovery manager (Sprint 5.5)

**Phase 5 Sprint 5.3: NAT Traversal - STUN/ICE (2025-11-30):**
- Implemented STUN client for NAT type detection (RFC 5389)
  - `StunClient` with async STUN binding request/response
  - NAT type detection (Full Cone, Restricted Cone, Port-Restricted Cone, Symmetric)
  - Public IP and port mapping discovery
  - Multiple STUN server support for reliability
  - Transaction ID tracking for request/response correlation
  - Timeout handling and retry logic
  - 9 comprehensive tests
- Added ICE candidate gathering
  - `IceCandidate` types (Host, ServerReflexive, Relayed)
  - Candidate priority calculation
  - Foundation and component ID generation
  - `IceAgent` for candidate collection and management
  - Integration with STUN client for reflexive candidates
  - 6 comprehensive tests
- Implemented UDP hole punching
  - Simultaneous open technique for NAT traversal
  - Hole punch attempt tracking and coordination
  - Success/failure callback support
  - Integration with ICE candidate gathering
  - 4 comprehensive tests
- **Test Results:** 19 new unit tests, all passing
- **Quality Gates:** All passing (fmt, clippy, test)
- **Progress:** Phase 5 Sprint 5.3 Complete (89/123 SP, 72% of Phase 5)

**Phase 5 Sprint 5.2: DHT Core - Kademlia (2025-11-30):**
- Implemented Kademlia DHT with privacy enhancements
  - `NodeId` based on BLAKE3 hash (256-bit cryptographic identifiers)
  - XOR-distance metric for routing
  - `KBucket` routing table with k=20 bucket size
  - Peer information tracking (NodeId, address, last seen)
  - K-closest nodes selection algorithm
  - Bucket splitting and eviction policies
  - 12 comprehensive tests
- Added DHT RPC operations
  - `DhtMessage` protocol with 4 RPC types:
    - PING: Liveness check
    - FIND_NODE: Locate k-closest nodes to target ID
    - STORE: Store key-value pairs
    - FIND_VALUE: Retrieve stored values
  - Request/response correlation with transaction IDs
  - Comprehensive serialization/deserialization
  - 8 comprehensive tests
- Implemented `DhtNode` for DHT operations
  - Peer discovery via FIND_NODE queries
  - Value storage and retrieval
  - Routing table maintenance
  - Bootstrap node integration
  - Periodic refresh and cleanup
  - 14 comprehensive tests
- **Test Results:** 34 new unit tests, all passing (total: 74 transport tests)
- **Quality Gates:** All passing (fmt, clippy, test)
- **Progress:** Phase 5 Sprint 5.2 Complete (55/123 SP, 45% of Phase 5)

**Phase 5 Sprint 5.4: Relay Infrastructure (2025-11-30):**
- Implemented DERP-style relay infrastructure for NAT traversal
  - `RelayMessage` protocol with 9 message types (Register, SendPacket, RecvPacket, etc.)
  - Comprehensive serialization/deserialization with bincode
  - End-to-end encryption (relay cannot decrypt payloads)
- Added `RelayClient` for connecting to relay servers
  - Async registration with relay servers
  - Packet forwarding through relay
  - Automatic keepalive mechanism
  - State machine (Disconnected, Connecting, Registering, Connected, Error)
  - Background message receiver task
  - 4 comprehensive tests
- Implemented `RelayServer` skeleton
  - Client registration and connection management
  - Packet forwarding between peers
  - Rate limiting (configurable packets per client per second)
  - Client timeout and automatic cleanup
  - Connection statistics tracking
  - Configurable max clients, rate limits, timeouts
  - 6 comprehensive tests
- Added `RelaySelector` with intelligent relay selection
  - Multiple selection strategies (LowestLatency, LowestLoad, HighestPriority, Balanced)
  - Geographic region filtering
  - Latency measurement and tracking
  - Load balancing across relays
  - Fallback relay ordering
  - 14 comprehensive tests
- Integration tests: 10 new tests covering full relay workflow
- **Test Results:** 37 new unit tests + 10 integration tests, all passing (47 total)
- **Quality Gates:** All passing (fmt, clippy, test)
- **Progress:** Phase 5 Sprint 5.4 Complete (110/123 SP, 89% of Phase 5)

**Phase 5 Sprint 5.1: Transport Trait Abstraction (2025-11-30):**
- Implemented `Transport` trait for multi-backend transport abstraction
  - Async send/receive operations with proper error handling
  - Transport statistics tracking (bytes, packets, errors)
  - Graceful shutdown support with `close()` and `is_closed()`
- Added `AsyncUdpTransport` - Tokio-based async UDP implementation
  - Implements `Transport` trait with full statistics
  - Optimized 2MB socket buffers for high-throughput
  - Comprehensive test coverage (6 tests, all passing)
- Added `QuicTransport` placeholder for future QUIC support
  - Proper error messages indicating not-yet-implemented status
  - Placeholder tests for future implementation
- Implemented `TransportFactory` pattern
  - Configuration-based transport creation
  - Support for UDP (implemented) and QUIC (placeholder)
  - Helper methods: `create_udp()`, `create_quic()`
  - Transport availability checking
- Dependencies: Added `async-trait = "0.1"` for async trait support
- **Test Results:** 8 new tests, all passing (74 total transport tests)
- **Quality Gates:** All passing (fmt, clippy, test)
- **Progress:** Phase 5 Sprint 5.1 Complete (21/123 SP, 17% of Phase 5)

---

## [0.4.8] - 2025-11-30

### Added

**P0 Critical Security Hardening (2025-11-30):**
- Complete `unsafe` code documentation audit across all crates
- Documented all `unsafe impl Send/Sync` implementations:
  - `wraith-transport::Umem` - SAFETY: Single owner, no shared mutable access
  - `wraith-transport::AfXdpSocket` - SAFETY: Atomic operations ensure thread safety
  - `wraith-xdp::XdpProgram` - SAFETY: No concurrent access, immutable after load
- Added comprehensive SAFETY comments to 40+ unsafe blocks
  - `wraith-transport::af_xdp.rs` - 22 SAFETY comments (UMEM, ring ops, packet data access)
  - `wraith-transport::numa.rs` - 9 SAFETY comments (mbind, topology detection)
  - `wraith-transport::worker.rs` - 5 SAFETY comments (CPU affinity, core pinning)
  - `wraith-files::io_uring_impl.rs` - 4 SAFETY comments (io_uring operations)
- 100% unsafe code documentation coverage achieved
- All unsafe code now has:
  - Detailed justification explaining why unsafe is necessary
  - Safety invariants documented
  - Precondition requirements stated
  - Thread safety analysis where applicable

**Security Scanning Infrastructure:**
- Added `.gitleaks.toml` configuration for false positive suppression
  - Test vectors allowlist (BLAKE3, XChaCha20-Poly1305, X25519, Ed25519)
  - Documentation examples allowlist (key formats, protocol examples)
  - Zero real security findings after allowlist application
- Gitleaks integrated into security scanning workflow
- All security gates passing (CodeQL, cargo-audit, gitleaks)

**Pre-Phase 5 Technical Debt Review (2025-11-30):**
- Comprehensive pre-Phase 5 technical debt review (3 files, 753 insertions)
- `to-dos/technical-debt/pre-phase-5-review-summary.md` - Executive summary of readiness assessment
- `to-dos/technical-debt/IMPLEMENTATION-REPORT.md` - Detailed implementation findings for all 15 items
- `to-dos/technical-debt/phase-4-tech-debt.md` - Updated with review completion status
- **Zero blocking items for Phase 5** - All critical quality gates passed
- 15 technical debt items analyzed: 4 complete, 1 executed, 10 deferred
- Code quality items from v0.3.1 verified complete (#[must_use], doc backticks, error documentation)
- Performance benchmarks validated (172M frames/sec, 3.2 GB/s AEAD, 8.5 GB/s BLAKE3)
- Test coverage confirmed at 607 tests (100% pass rate)
- Phase 5 readiness statement confirmed across all crates

**Technical Debt Tracking Documentation:**
- Comprehensive technical debt analysis (92/100 quality score, Grade A)
- Technical debt action plan with prioritized remediation strategies
- Technical debt TODO list for actionable tracking
- Protocol comparison document (WRAITH vs WireGuard, QUIC, Tor, I2P)

**Documentation Files Added:**
- `to-dos/technical-debt/technical-debt-analysis.md` (~40 KB) - Complete code quality assessment
- `to-dos/technical-debt/technical-debt-action-plan.md` (~25 KB) - Strategic remediation plan
- `to-dos/technical-debt/technical-debt-todo-list.md` (~20 KB) - Actionable tracking checklist
- `ref-docs/WRAITH-Protocol-Comparison-v1.0.md` (~85 KB) - Comprehensive protocol comparison

**Code Quality Metrics:**
- Quality Grade: A (92/100)
- Technical Debt Ratio: 14% (within healthy range)
- Test Coverage: 607 tests passing (100% pass rate)
- Security Vulnerabilities: Zero
- Clippy Warnings: Zero
- Documentation: Complete technical debt tracking framework (6 files)

**Technical Debt Items Tracked:**
- 15 analyzed items (4 complete, 1 executed, 10 deferred)
- Effort estimates: 14-22 hours total for remaining items
- Priority classification: P0 (2 complete), P1 (4), P2 (3), P3 (2)
- Impact assessment: Low to Medium
- Risk level: Low
- **Phase 5 Readiness:** Zero blocking items

### Changed

**Documentation:**
- Updated README.md with pre-Phase 5 readiness status
- Added Phase 5 readiness confirmation to Current Status section
- Updated Technical Debt & Quality section with new review documentation
- Enhanced test coverage metrics (555+ → 607 tests)
- Updated Code Quality Metrics with Phase 5 readiness statement
- Enhanced bottom status line with Phase 5 readiness and zero blocking items

---

## [0.4.5] - 2024-11-30

### Added
- Comprehensive Phase 4 documentation in README
- Privacy & Obfuscation documentation section
- Cross-platform build support documentation

### Fixed
- Windows x86_64-pc-windows-msvc cross-platform compatibility
  - Platform-specific RawFd type handling in io_uring module
  - Added cfg attributes for Unix vs Windows builds
- CI MSRV (Rust 1.85) build failure
  - Enabled getrandom feature for rand_core dependency
  - Resolved OsRng import and trait mismatch errors
- Useless unsigned comparison warning in timing tests

### Changed
- Updated test coverage documentation (607 tests)
- Enhanced README with complete obfuscation feature details
- Improved release artifact naming and organization

---

## [0.4.0] - 2024-11-30

### Added

**Phase 4 Part II - Obfuscation & Stealth - COMPLETE ✅ (2024-11-30):**

This release completes Phase 4 Part II, delivering comprehensive traffic obfuscation with packet padding, timing obfuscation, cover traffic generation, and protocol mimicry to defeat deep packet inspection and traffic analysis.

#### Packet Padding Engine (Sprint 4.1, 21 SP)

Complete packet padding implementation with 5 modes and adaptive selection:

- **5 Padding Modes**:
  - `None` - No padding (maximum performance)
  - `PowerOfTwo` - Round to next power of 2 (15% overhead)
  - `SizeClasses` - Fixed size classes: 128, 512, 1024, 4096, 8192, 16384 bytes (10% overhead)
  - `ConstantRate` - Always maximum size (50% overhead, maximum privacy)
  - `Statistical` - Geometric distribution-based random padding (20% overhead)
- **Adaptive Profile Selection**: Automatic mode selection based on threat level (Low, Medium, High, Paranoid)
- **Overhead Estimation**: Real-time overhead calculation for each mode
- **30 comprehensive tests** covering all padding modes and adaptive selection

#### Timing Obfuscation (Sprint 4.2, 13 SP)

Advanced timing obfuscation with 5 distribution modes and traffic shaping:

- **5 Timing Modes**:
  - `None` - No delay (baseline)
  - `Fixed` - Constant delay
  - `Uniform` - Uniform random distribution
  - `Normal` - Normal (Gaussian) distribution
  - `Exponential` - Exponential distribution (Poisson process simulation)
- **Traffic Shaper**: Rate-controlled packet timing with configurable PPS limits
- **Statistical Distributions**: Integration with `rand_distr` for authentic traffic patterns
- **29 comprehensive tests** including distribution validation and traffic shaping

#### Protocol Mimicry (Sprint 4.3, 34 SP)

Three complete protocol wrappers for traffic obfuscation:

- **TLS 1.3 Record Layer Mimicry** (20 tests):
  - Application data wrapping with authentic TLS 1.3 records
  - Fake handshake generation (ClientHello, ServerHello, Finished)
  - Sequence number tracking for realistic sessions
  - Content type 23 (application_data) with version 0x0303
- **WebSocket Binary Frame Wrapping** (21 tests):
  - Binary frame encoding with FIN bit and opcode 0x02
  - Client masking support with random masking keys
  - Extended length encoding (126 for 16-bit, 127 for 64-bit lengths)
  - Payload masking XOR operation
- **DNS-over-HTTPS Tunneling** (22 tests):
  - base64url encoding for DNS query parameters
  - DNS query packet generation with EDNS0 OPT records
  - Payload embedding in EDNS data field
  - Query/response parsing with comprehensive validation

#### Testing & Benchmarks (Sprint 4.4, 8 SP)

- **130 unit tests** across all obfuscation modules
- **37 doctests** for API documentation examples
- **Criterion benchmarks**:
  - Padding engine performance (all 5 modes)
  - TLS record wrapping/unwrapping
  - WebSocket frame operations
  - DoH tunnel encoding/decoding
  - Timing obfuscator delay generation
- **Total test coverage**: 167 tests passing

**Quality Gates:**
- ✅ All 597 workspace tests passing
- ✅ Clippy clean (zero warnings)
- ✅ rustfmt compliant
- ✅ Comprehensive documentation with examples

### Fixed

**Cross-Platform Compatibility:**
- **Windows x86_64-pc-windows-msvc Support**: Fixed `RawFd` type handling in `wraith-transport`
  - Added platform-specific type definitions for Windows compatibility
  - `RawFd` now conditionally defined as `c_int` on Unix and `isize` on Windows
  - Enables successful cross-platform builds for Windows targets (commit: 88ba377)
  - Maintains zero-cost abstraction on all platforms
  - Resolves compilation errors when building for Windows MSVC targets

**CI/CD Build Improvements:**
- **MSRV Build Fix**: Enabled `getrandom` feature for `rand_core` dependency
  - Resolves "getrandom" function not found error in Rust 1.85 MSRV builds
  - Ensures CI MSRV verification workflow passes consistently
  - Maintains compatibility with minimum supported Rust version (1.85)
  - No impact on runtime performance or security
  - Fix applied in Cargo.toml for wraith-obfuscation crate

**Timing Test Warnings:**
- **Useless Unsigned Comparison Warning**: Fixed in `wraith-obfuscation` timing tests (commit: 88ba377)
  - Removed redundant >= 0 comparison for Duration values (always unsigned)
  - Eliminated clippy warning without changing test behavior
  - Improved code quality and maintainability

### Changed

**Documentation:**
- **README.md**: Comprehensive update with complete Phase 4 status
  - Progress metrics: 499/789 story points (63% overall completion)
  - Test count: 607 passing tests (detailed breakdown by crate)
  - Code volume: ~21,000+ lines of Rust across all crates
  - Enhanced Privacy & Obfuscation section with all 5 padding modes and 5 timing distributions
  - Added complete protocol mimicry documentation (TLS 1.3, WebSocket, DoH)
  - Updated Security section with obfuscation test coverage
  - Added cross-platform support details (Linux, macOS, Windows)
  - Performance metrics: frame parsing, AEAD encryption, BLAKE3 hashing
  - Updated Current Focus Areas with Phase 4 Part II completion
- **CHANGELOG.md**: Complete Phase 4 documentation
  - Detailed Sprint 4.1-4.4 deliverables (padding, timing, mimicry, testing)
  - Cross-platform compatibility fixes (Windows MSVC, MSRV build)
  - CI/CD improvements and warning resolutions
  - Test coverage progression (487 → 607)
- **Test Breakdown**: Accurate counts with unit + doctest separation
  - wraith-core: 197 tests
  - wraith-crypto: 123 tests (1 ignored)
  - wraith-transport: 54 tests (1 ignored)
  - wraith-obfuscation: 167 tests (130 unit + 37 doctests)
  - wraith-files: 16 tests (12 unit + 4 doctests)
  - Integration vectors: 24 tests
  - Integration tests: 15 tests
  - Total: 607 tests (52 doctests + 555 unit/integration tests)

---

**Phase 4 Part I - Optimization & Hardening - COMPLETE ✅ (2025-11-30):**

This release completes Phase 4 Part I, delivering high-performance kernel bypass features and comprehensive security hardening across the entire protocol stack.

#### AF_XDP Zero-Copy Socket Implementation (Sprints 4.1-4.2, PERF-001)

Complete Linux AF_XDP integration for kernel bypass networking with zero-copy packet I/O:

- **UMEM Management**: User-space memory allocation with configurable frame sizes (2048/4096 bytes)
- **Four-Ring Architecture**:
  - Fill Ring: Kernel → User packet delivery
  - RX Ring: Received packet descriptors
  - TX Ring: Transmit packet descriptors
  - Completion Ring: TX completion notifications
- **Producer/Consumer Synchronization**: Lock-free ring operations with atomic indices
- **Batch Processing APIs**:
  - `rx_batch()` - Receive multiple packets in a single call
  - `tx_batch()` - Submit multiple packets for transmission
  - `complete_tx()` - Collect transmission completions
  - `fill_rx_buffers()` - Replenish receive buffers
- **Zero-Copy Packet Access**: Direct buffer access via `get_packet_data()` and `get_packet_data_mut_unsafe()`
- **16 comprehensive tests** covering all ring operations and edge cases

**Performance Target:** 10-40 Gbps with compatible NICs

#### BBR Pacing Enforcement (Sprint 4.3, PERF-002)

Timer-based pacing rate enforcement integrated with BBR congestion control:

- **Credit Accumulation System**: Smooth packet transmission without bursts
- **Phase-Specific Pacing Gains**:
  - Startup: 2.77x (aggressive bandwidth probing)
  - Drain: 2.0x (queue draining after startup)
  - ProbeBw: 8-phase cycle [1.25, 0.75, 1, 1, 1, 1, 1, 1]
  - ProbeRtt: 1.0x (RTT measurement mode)
- **Pacing APIs**:
  - `can_send_paced()` - Check if sending is allowed
  - `on_packet_sent_paced()` - Update pacing state after send
  - `pacing_delay()` - Get delay until next send allowed
- **Dynamic Rate Updates**: Pacing rate adjusts based on BBR bandwidth estimate and phase
- **Burst Prevention**: Credit system prevents packet bursts that could trigger congestion
- **3 comprehensive tests** for pacing behavior validation

**Performance Target:** <5% transmission jitter

#### io_uring Async File I/O (Sprint 4.4, PERF-003)

Linux io_uring integration for high-performance async file operations:

- **Async Operations**: Non-blocking read, write, and fsync
- **Registered Buffers**: Zero-copy I/O with pre-registered memory regions
- **Batch Submission**: Multiple operations submitted per syscall
- **Configurable Queue Depth**: 128-4096 for batched operations
- **High-Level APIs**:
  - `AsyncFileReader` - Streaming file reads with automatic batching
  - `AsyncFileWriter` - Buffered file writes with configurable flush
- **Completion Tracking**: Request ID mapping for async operation completion
- **Platform Fallback**: Synchronous I/O implementation for non-Linux systems
- **15 comprehensive tests** covering all I/O operations and edge cases

**Performance Target:** >100K IOPS

#### Frame Validation Hardening (Sprint 4.5, SEC-001)

Comprehensive input validation for protocol frames to prevent attacks:

- **Reserved Stream ID Validation**: Stream IDs 1-15 reserved for protocol control use
  - Prevents application usage of reserved stream IDs
  - Ensures protocol integrity for control streams
- **Offset Bounds Checking**: Maximum file offset 256 TB (2^48 bytes)
  - Prevents integer overflow attacks
  - Validates offset + length combinations
- **Payload Size Limits**: Maximum 8,944 bytes (9000 MTU - 28 header - 16 auth tag)
  - Enforces MTU constraints
  - Prevents memory exhaustion attacks
- **New Error Types**:
  - `ReservedStreamId(u32)` - Application attempted to use reserved stream ID
  - `InvalidOffset { offset, max }` - Offset exceeds protocol maximum
  - `PayloadTooLarge { size, max }` - Payload exceeds MTU limit
- **Validation Constants**:
  - `MAX_PAYLOAD_SIZE = 8944` (9000 - 28 - 16)
  - `MAX_FILE_OFFSET = 281474976710656` (2^48)
  - `MAX_SEQUENCE_DELTA = 4294967295` (2^32 - 1)
- **Property-Based Testing**: Using proptest for fuzzing validation logic
- **13 comprehensive tests** including edge cases and manual frame corruption

#### Buffer Pool & Documentation (Sprint 4.6, PERF-004, DOC-001)

- **Global Buffer Pool** (already implemented in wraith-crypto):
  - Thread-safe buffer reuse for encryption hot path
  - Lock-free allocation with `BufferPool` type
  - Integration via `encrypt_with_pool()` and `decrypt_with_pool()`
  - Reduces allocation overhead in packet processing
- **Complete Frame Type Documentation**:
  - Documented all 15 frame types in `ref-docs/protocol_technical_details.md`
  - Added missing frame type specifications:
    - STREAM_CLOSE (0x0A) - Stream termination with optional error code
    - STREAM_RESET (0x0B) - Abrupt stream abort with error code
    - WINDOW_UPDATE (0x0C) - Flow control window increment
    - GO_AWAY (0x0D) - Connection migration to new path
    - PATH_CHALLENGE (0x0E) - Path validation request with nonce
    - PATH_RESPONSE (0x0F) - Path validation response with echoed nonce
  - Complete payload layouts with field descriptions
  - Behavior specifications for each frame type
  - Integration examples with session and stream layers

### Changed

- **Test Updates**:
  - Updated all tests to use stream ID 16+ (avoiding newly reserved range 1-15)
  - Fixed integration tests to comply with new validation rules
  - Updated property-based tests to generate only valid parameters
  - Total tests increased to **487 passing tests** (Phase 4 added 49 new tests)
- **Test Breakdown**:
  - wraith-core: 197 tests (frame, session, stream, BBR, path, migration)
  - wraith-crypto: 123 tests (AEAD, signatures, hashing, Noise, ratchet, constant-time)
  - wraith-transport: 54 tests (AF_XDP, io_uring, UDP, MTU, worker pools)
  - wraith-obfuscation: 47 tests (padding, timing, cover traffic)
  - wraith-files: 12 tests (chunking, hashing, async I/O)
  - Integration vectors: 24 tests (cryptographic correctness)
  - Integration tests: 15 tests (session crypto, frame encryption)
  - Doctests: 15 tests (API examples)
- **Quality Improvements**:
  - All code passes `cargo clippy --workspace -- -D warnings` (zero warnings)
  - All code formatted with `cargo fmt --all`
  - Documentation builds successfully without warnings
  - Zero test failures across all workspace crates

### Performance

- **Frame Parsing**: 172M frames/sec (5.8ns/frame, 232 GiB/s theoretical throughput)
- **AEAD Encryption**: 3.2 GB/s (single core)
- **BLAKE3 Hashing**: 8.5 GB/s (parallel)
- **Session Creation**: 45μs average
- **AF_XDP Zero-Copy**: 10-40 Gbps target with compatible NICs
- **io_uring Async I/O**: >100K IOPS target

### Security

- **Input Validation**: Reserved stream IDs, offset bounds, payload size limits
- **Zero Unsafe Code**: All cryptographic paths remain free of unsafe blocks
- **Constant-Time Operations**: All critical comparisons use constant-time functions
- **Memory Zeroization**: Automatic cleanup of sensitive key material
- **Test Coverage**: 487 tests covering security-critical paths

### Documentation

- **Frame Type Specifications**: All 15 frame types fully documented
- **Protocol Reference**: Complete wire format documentation
- **API Examples**: Comprehensive usage examples in doctests
- **Performance Benchmarks**: Updated with Phase 4 optimizations

---

## [0.3.2] - 2025-11-30

### Added

**Phase 4 Part I - Optimization & Hardening (Sprints 4.1-4.6) - COMPLETE ✅ (2025-11-30):**

- **AF_XDP Socket Implementation** (Sprint 4.1-4.2, PERF-001) ✅
  - Complete zero-copy packet I/O with AF_XDP on Linux
  - UMEM (User-space Memory) management with configurable frame sizes (2048/4096 bytes)
  - Fill, RX, TX, and Completion ring implementations with producer/consumer indices
  - Batch packet processing APIs (`rx_batch`, `tx_batch`, `complete_tx`, `fill_rx_buffers`)
  - Zero-copy packet data access with `get_packet_data()` and `get_packet_data_mut_unsafe()`
  - Comprehensive test suite (16 tests covering all ring operations and edge cases)

- **BBR Pacing Enforcement** (Sprint 4.3, PERF-002) ✅
  - Timer-based pacing rate enforcement in congestion control
  - `can_send_paced()` and `on_packet_sent_paced()` APIs with credit accumulation
  - `pacing_delay()` calculation for inter-packet timing
  - Dynamic pacing rate updates based on BBR phase (Startup: 2.77x, Drain: 2.0x, ProbeBw: cycle, ProbeRtt: 1.0x)
  - Integration with existing 4-phase BBR state machine
  - Added 3 comprehensive pacing tests (enforcement, delay calculation, burst prevention)

- **io_uring File I/O Integration** (Sprint 4.4, PERF-003) ✅
  - Async file I/O using Linux io_uring (kernel 5.1+)
  - Support for read, write, and fsync operations with batching
  - Registered buffer support for zero-copy I/O (`register_buffers`)
  - Batched operation submission and completion polling (`submit_batch`, `wait_completions`)
  - Platform-independent API with high-level `AsyncFileReader` and `AsyncFileWriter`
  - 15 comprehensive tests covering all I/O operations and edge cases

- **Frame Validation Hardening** (Sprint 4.5, SEC-001) ✅
  - Reserved stream ID validation (IDs 1-15 now reserved for protocol control)
  - File offset bounds checking (max 256 TB = 2^48 bytes)
  - Payload size limits (max 8,944 bytes = 9000 MTU - 28 header - 16 auth tag)
  - Comprehensive validation constants (`MAX_PAYLOAD_SIZE`, `MAX_FILE_OFFSET`, `MAX_SEQUENCE_DELTA`)
  - Added `ReservedStreamId`, `InvalidOffset`, and `PayloadTooLarge` error variants
  - 13 comprehensive validation tests with manual frame corruption and property-based testing
  - Multiple validation failure handling (reports first encountered error)

- **Buffer Pool & Documentation** (Sprint 4.6, PERF-004, DOC-001) ✅
  - Global buffer pool already implemented in wraith-crypto (`BufferPool`)
  - Lock-free buffer allocation with reuse for zero-allocation hot path
  - Integration with `SessionCrypto` via `encrypt_with_pool()` and `decrypt_with_pool()`
  - Documented all 15 frame types in `ref-docs/protocol_technical_details.md`
  - Added missing frame type documentation: STREAM_CLOSE (0x0A), STREAM_RESET (0x0B), WINDOW_UPDATE (0x0C), GO_AWAY (0x0D), PATH_CHALLENGE (0x0E), PATH_RESPONSE (0x0F)
  - Complete protocol specification with payload layouts, field descriptions, and behavior specifications

### Changed

- **Test Updates:**
  - Updated all tests to use stream ID 16+ (avoiding newly reserved range 1-15)
  - Fixed integration tests to comply with new validation rules (reserved stream IDs, offset bounds)
  - Updated property-based tests to generate only valid parameters
  - Total tests increased to 487 passing tests (Phase 4 added 49 new tests)

- **Quality Improvements:**
  - All code passes `cargo clippy --workspace -- -D warnings` (zero warnings)
  - All code formatted with `cargo fmt --all`
  - Documentation builds successfully without warnings (`cargo doc --workspace`)
  - Zero test failures across all workspace crates
  - Test breakdown: wraith-core (197), wraith-crypto (123), integration vectors (24), wraith-files (12), integration tests (15), wraith-obfuscation (47), wraith-transport (54), doctests (15)
  - Total: **487 passing tests** across all crates

### Fixed

- Property-based tests now generate valid frame parameters (stream IDs, offsets, payload sizes)
- Integration tests updated for reserved stream ID range

---

## [0.3.1] - 2025-11-30

### Changed

**Code Quality and Style Improvements:**
- Added `#[must_use]` attributes to ~65 pure functions across wraith-core and wraith-crypto
  - Ensures function results are not accidentally discarded
  - Improves API safety and developer ergonomics
- Enhanced documentation with proper backticks for technical terms
  - Improved rustdoc rendering and code example clarity
- Added comprehensive `# Errors` documentation to Result-returning functions
- Added `# Panics` documentation where applicable
- Improved test coverage from 402 to 438 tests (+36 tests, +9% increase)

**Technical Debt Remediation:**
- Addressed immediate code quality issues identified during Phase 3 review
- Removed duplicate `io_uring_impl.rs` file in wraith-files
- Added comprehensive SAFETY comments for unsafe code justifications
- Fixed pattern matching redundancy in noise.rs
- Improved constant-time operation documentation
- Enhanced error handling documentation across all public APIs

**Test Suite Enhancements:**
- wraith-transport: Increased from 39 to 40 tests (worker pool queue full validation)
- wraith-obfuscation: Increased from 24 to 47 tests (+23 tests, timing and padding coverage)
- Integration vectors: Increased from 12 to 24 tests (+12 tests, cryptographic correctness)
- Total test count: 438 tests (177 core + 123 crypto + 24 vectors + 12 files + 15 integration + 47 obfuscation + 40 transport)

**Documentation Updates:**
- Updated README.md with accurate test counts and implementation status
- Updated line count from ~15,000 to ~16,500 lines of Rust code
- Corrected test breakdown across all crates
- Enhanced security validation documentation

### Fixed
- Documentation formatting inconsistencies across multiple modules
- Missing assertions in worker pool queue full test

### Performance
- Maintained 172M frames/sec parsing performance (232 GiB/s theoretical throughput)
- Zero performance regression from code quality improvements
- All benchmarks stable across refactoring

### Quality
- Zero clippy errors maintained
- Zero unsafe code in cryptographic paths
- All tests passing (438/438)
- Documentation coverage improved

---

## [0.3.0] - 2025-11-30

### Added

**Phase 3: Transport & Kernel Bypass - COMPLETE ✅ (2025-11-30):**

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

### Phase 3 Deliverables ✅

**Completed Components (156/156 story points):**
1. ✅ XDP/eBPF packet filter with AF_XDP socket redirection
2. ✅ AF_XDP socket management with UMEM and ring buffers
3. ✅ Worker thread pool with CPU core pinning and per-worker statistics
4. ✅ NUMA-aware memory allocation and topology detection
5. ✅ Path MTU discovery with binary search probing
6. ✅ UDP transport with cross-platform support
7. ✅ io_uring async file I/O with batched operations
8. ✅ Transport benchmarks (UDP throughput, latency, worker pools, MTU cache)
9. ✅ Comprehensive test suite (39 transport tests, 12 files tests)
10. ✅ Cross-platform graceful fallbacks (non-Linux stubs)

**Performance Validation:**
- ✅ AF_XDP zero-copy framework operational
- ✅ io_uring async I/O with queue depth 128-4096
- ✅ UDP transport with 2MB buffers for high throughput
- ✅ Worker pool with configurable thread count and core pinning
- ✅ All quality gates passing (clippy, fmt, tests)

**Documentation:**
- XDP/eBPF implementation details
- AF_XDP socket management patterns
- Worker thread pool architecture
- NUMA allocation strategies
- Transport benchmark results

**Next: Phase 4 - Obfuscation & Stealth**

**Prerequisites Met:**
- Transport layer operational ✅
- Kernel bypass framework ready ✅
- Async I/O integrated ✅
- Cross-platform support confirmed ✅

**Phase 4 Focus (76 story points, 3-4 weeks):**
- Protocol mimicry (TLS, WebSocket, DNS-over-HTTPS wrappers)
- Advanced padding strategies
- Timing obfuscation with jitter
- Covert channel support

---

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

[Unreleased]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.5.5...v0.6.0
[0.5.5]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.5.0...v0.5.5
[0.5.0]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.4.8...v0.5.0
[0.4.8]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.4.5...v0.4.8
[0.4.5]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.4.0...v0.4.5
[0.4.0]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.1.0...v0.1.5
[0.1.0]: https://github.com/doublegate/WRAITH-Protocol/releases/tag/v0.1.0
