# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WRAITH (Wire-speed Resilient Authenticated Invisible Transfer Handler) is a decentralized secure file transfer protocol. This repository contains the Rust implementation along with design specifications.

**Current Status:** Version 1.5.0 - WRAITH Transfer Desktop Application (Phase 15 Complete)

**Current Metrics:**
- **Tests:** 1,303 tests total (1,280 passing, 23 ignored) - 100% pass rate on active tests
- **Code Volume:** ~41,177 lines of Rust code (~30,876 LOC + 2,743 comments + 7,558 blanks) across 7 active crates
- **Documentation:** 100+ files, 35,000+ lines including tutorial, integration guide, troubleshooting, security audit, protocol comparison, reference client design, architecture docs, API reference, performance report, release notes, error handling audit
- **Security:** Zero vulnerabilities, EXCELLENT security posture ([v1.1.0 audit](docs/security/SECURITY_AUDIT_v1.1.0.md), 286 dependencies scanned), 100% unsafe block documentation
- **Performance:** File chunking 14.85 GiB/s, tree hashing 4.71 GiB/s, chunk verification 4.78 GiB/s, file reassembly 5.42 GiB/s (Phase 10/12 benchmarks)
- **Quality:** Code quality 98/100, technical debt 3.8%, zero clippy warnings, compile-time address construction

## Build & Development Commands

```bash
# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Run lints
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt --all

# Run all CI checks
cargo xtask ci

# Build release
cargo build --release

# Generate documentation
cargo doc --workspace --open

# Run the CLI
cargo run -p wraith-cli -- --help
```

## Repository Structure

```
WRAITH-Protocol/
├── crates/                 # Rust workspace crates
│   ├── wraith-core/        # Frame encoding, session state, congestion control, Node API
│   ├── wraith-crypto/      # Noise handshake, AEAD, Elligator2, ratcheting
│   ├── wraith-transport/   # AF_XDP, io_uring, UDP sockets
│   ├── wraith-obfuscation/ # Padding, timing, protocol mimicry
│   ├── wraith-discovery/   # DHT, relay, NAT traversal
│   ├── wraith-files/       # Chunking, integrity, transfer state
│   ├── wraith-cli/         # Command-line interface (wraith binary)
│   └── wraith-xdp/         # eBPF/XDP programs (Linux-only, excluded from default build)
├── xtask/                  # Build automation (cargo xtask <cmd>)
├── docs/                   # Documentation
│   ├── archive/            # Archived/outdated documentation
│   ├── architecture/       # Architecture documentation
│   ├── cli/                # CLI documentation and guides
│   ├── clients/            # Client application specs
│   ├── engineering/        # API reference, coding standards, release notes
│   ├── integration/        # Integration guides
│   ├── operations/         # Operations and deployment guides
│   ├── progress/           # Development progress tracking
│   ├── runbooks/           # Operational runbooks
│   ├── security/           # Security audits and documentation
│   ├── technical/          # Technical debt analysis, refactoring docs
│   ├── testing/            # Testing guides and strategies
│   ├── xdp/                # XDP/eBPF documentation
│   ├── COMPARISON.md       # Protocol comparison (QUIC, WireGuard, BitTorrent)
│   ├── CONFIG_REFERENCE.md # Configuration reference
│   ├── INTEGRATION_GUIDE.md # Developer integration guide
│   ├── PERFORMANCE_REPORT.md # Performance benchmarks and analysis
│   ├── SECURITY_AUDIT.md   # Latest security audit report
│   ├── TROUBLESHOOTING.md  # Troubleshooting guide
│   ├── TUTORIAL.md         # Getting started tutorial
│   └── USER_GUIDE.md       # End-user guide
├── to-dos/                 # Project planning and task tracking
│   ├── protocol/           # Phase planning and progress documents
│   ├── completed/          # Completed phase summaries
│   ├── technical-debt/     # Technical debt tracking
│   ├── ROADMAP.md          # Project roadmap
│   └── ROADMAP-clients.md  # Client applications roadmap
├── ref-docs/               # Protocol specifications
│   ├── protocol_technical_details.md
│   └── protocol_implementation_guide.md
├── images/                 # Branding assets
├── tests/                  # Integration tests
└── benches/                # Benchmarks
```

## Protocol Architecture

Six-layer design (bottom to top):
1. **Network Layer** - UDP, raw sockets, covert channels
2. **Kernel Acceleration** - AF_XDP, io_uring, zero-copy DMA
3. **Obfuscation Layer** - Elligator2, padding, timing jitter
4. **Crypto Transport** - Noise_XX, XChaCha20-Poly1305, ratcheting
5. **Session Layer** - Stream mux, flow control, BBR congestion
6. **Application Layer** - File transfer, chunking, integrity

## Key Technical Details

### Cryptographic Suite
- **Key Exchange:** X25519 with Elligator2 encoding
- **AEAD:** XChaCha20-Poly1305 (192-bit nonce)
- **Hash:** BLAKE3 (tree-parallelizable)
- **Handshake:** Noise_XX (mutual auth, identity hiding)

### Wire Format
- **Outer Packet:** 8B CID + encrypted payload + 16B auth tag
- **Inner Frame:** 28B header + payload + random padding
- **Frame Types:** DATA, ACK, CONTROL, REKEY, PING/PONG, CLOSE, PAD, STREAM_*, PATH_*

### Performance Targets
- Throughput: 300+ Mbps (10-40 Gbps with kernel bypass)
- Latency: Sub-millisecond with AF_XDP
- Forward secrecy: Ratchet every 2 min or 1M packets

## Development Notes

### Target Platform
- Linux 6.2+ (for AF_XDP, io_uring)
- Primary: x86_64, Secondary: aarch64
- Rust 1.85+ (2024 Edition, MSRV: 1.85)

### Key Dependencies
- `chacha20poly1305`, `x25519-dalek`, `blake3` - Cryptography
- `snow` - Noise Protocol framework
- `io-uring` - Async file I/O (Linux)
- `tokio` - Async runtime
- `clap` - CLI parsing

### Threading Model
Thread-per-core with no locks in hot path. Sessions pinned to cores, NUMA-aware allocation.

## Implementation Status

| Crate | Status | Tests | Notes |
|-------|--------|-------|-------|
| wraith-core | ✅ Complete | 406 | Frame parsing (SIMD), sessions, streams, BBR, migration, Node API orchestration, health monitoring |
| wraith-crypto | ✅ Complete | 128 | Ed25519, X25519+Elligator2, XChaCha20-Poly1305, BLAKE3, Noise_XX, Double Ratchet |
| wraith-transport | ✅ Complete | 88 | AF_XDP zero-copy, io_uring, UDP, worker pools, NUMA-aware, buffer pool |
| wraith-obfuscation | ✅ Complete | 130 | Padding (5 modes), timing (5 distributions), TLS/WebSocket/DoH mimicry |
| wraith-discovery | ✅ Complete | 154 | Privacy-enhanced Kademlia DHT, STUN, ICE, DERP-style relay |
| wraith-files | ✅ Complete | 34 | io_uring file I/O, chunking, BLAKE3 tree hashing, reassembly |
| wraith-cli | ✅ Complete | 7 | Full CLI with config, progress display, send/receive/daemon commands |
| wraith-xdp | Not started | 0 | Requires eBPF toolchain (future phase) |

**Total:** 1,296 tests across all crates and integration tests (1,280 passing, 16 ignored)
