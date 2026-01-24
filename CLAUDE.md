# CLAUDE.md - WRAITH Protocol

Guidance for Claude Code when working with this repository.

## Project Overview

WRAITH (Wire-speed Resilient Authenticated Invisible Transfer Handler) is a decentralized secure file transfer protocol implemented in Rust.

**Status:** v2.0.1 - Bug Fixes & UI Standardization

### Metrics
| Metric | Value |
|--------|-------|
| Tests | 1,695 passing (16 ignored) - 100% pass rate |
| Code | ~68,000 lines Rust (protocol) + ~12,000 lines (clients) across 11 crates |
| Documentation | 120+ files, 72,000+ lines |
| Security | Zero vulnerabilities - EXCELLENT ([v1.1.0 audit](docs/security/SECURITY_AUDIT_v1.1.0.md), 295 deps) |
| Performance | File chunking 14.85 GiB/s, tree hashing 4.71 GiB/s, verification 4.78 GiB/s, reassembly 5.42 GiB/s |
| Quality | 98/100, technical debt 3.5%, zero clippy warnings |

## Build & Development

```bash
cargo build --workspace           # Build
cargo test --workspace            # Test
cargo clippy --workspace -- -D warnings  # Lint
cargo fmt --all                   # Format
cargo xtask ci                    # All CI checks
cargo build --release             # Release build
cargo doc --workspace --open      # Documentation
cargo run -p wraith-cli -- --help # Run CLI
```

## Repository Structure

```
WRAITH-Protocol/
├── crates/                 # Rust workspace (7 active + 1 excluded)
│   ├── wraith-core/        # Frame, session, congestion, Node API
│   ├── wraith-crypto/      # Noise, AEAD, Elligator2, ratcheting
│   ├── wraith-transport/   # AF_XDP, io_uring, UDP
│   ├── wraith-obfuscation/ # Padding, timing, mimicry
│   ├── wraith-discovery/   # DHT, relay, NAT traversal
│   ├── wraith-files/       # Chunking, integrity, transfer
│   ├── wraith-cli/         # CLI (wraith binary)
│   └── wraith-xdp/         # eBPF/XDP (Linux-only, excluded)
├── clients/                # Desktop applications (Tauri)
├── xtask/                  # Build automation
├── docs/                   # Comprehensive documentation
│   ├── architecture/       # System architecture
│   ├── archive/            # Archived docs, backups
│   ├── clients/            # Client application specs
│   ├── engineering/        # API ref, coding standards, release notes
│   ├── integration/        # Integration guides
│   ├── operations/         # Operations and deployment
│   ├── security/           # Security audits
│   ├── technical/          # Technical debt analysis
│   ├── testing/            # Testing strategies
│   ├── troubleshooting/    # Tauri warnings and fixes
│   └── xdp/                # XDP/eBPF documentation
├── to-dos/                 # Project planning
│   ├── clients/            # Client sprint planning
│   ├── completed/          # Completed phases
│   ├── protocol/           # Protocol phase planning
│   └── technical-debt/     # Tech debt tracking
├── ref-docs/               # Protocol specifications
└── tests/benches/          # Integration tests & benchmarks

Root files (standard GitHub):
- README.md, CHANGELOG.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md
- CLAUDE.md (Claude Code instructions), CLAUDE.local.md (local state, .gitignored)
```

## Protocol Architecture

Six-layer design (bottom to top):
1. **Network** - UDP, raw sockets, covert channels
2. **Kernel Acceleration** - AF_XDP, io_uring, zero-copy DMA
3. **Obfuscation** - Elligator2, padding, timing jitter
4. **Crypto Transport** - Noise_XX, XChaCha20-Poly1305, ratcheting
5. **Session** - Stream mux, flow control, BBR congestion
6. **Application** - File transfer, chunking, integrity

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

| Crate | Status | Tests | Features |
|-------|--------|-------|----------|
| wraith-core | ✅ Complete | 414 | Frame (SIMD), Session, Stream, BBR, Migration, Node API |
| wraith-crypto | ✅ Complete | 127 | Ed25519, X25519+Elligator2, AEAD, Noise_XX, Ratchet |
| wraith-transport | ✅ Complete | 130 | AF_XDP, io_uring, UDP, worker pools, NUMA-aware |
| wraith-obfuscation | ✅ Complete | 111 | Padding (5), Timing (5), Mimicry (TLS/WebSocket/DoH) |
| wraith-discovery | ✅ Complete | 231 | Kademlia DHT, STUN, ICE, relay |
| wraith-files | ✅ Complete | 34 | io_uring I/O, chunking, tree hashing, reassembly |
| wraith-cli | ✅ Complete | 8 | Full CLI with config, progress, commands |
| wraith-ffi | ✅ Complete | 6 | C-compatible API, FFI-safe types |
| wraith-xdp | Not started | 0 | Requires eBPF toolchain (future) |

### Client Applications

| Client | Status | Tests | Features |
|--------|--------|-------|----------|
| wraith-transfer | ✅ Complete | 68 | Tauri desktop file transfer |
| wraith-chat | ✅ Complete | 76 | E2EE messaging, voice/video calls, groups |
| wraith-android | ✅ Complete | 96 | Kotlin + JNI, Keystore, FCM push |
| wraith-ios | ✅ Complete | 103 | Swift + UniFFI, Keychain, APNs push |

**Total:** 1,695 tests passing (16 ignored)
