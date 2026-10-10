# CLAUDE.md - WRAITH Protocol

Guidance for Claude Code when working with this repository.

## Project Overview

WRAITH (Wire-speed Resilient Authenticated Invisible Transfer Handler) is a decentralized secure file transfer protocol implemented in Rust.

**Status:** v2.3.7 - Testing Infrastructure & CI Stability

Current metrics (tests, performance, quality) live in `README.md` and `CHANGELOG.md`; they are not duplicated here.

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
├── crates/                 # Rust workspace (8 active + 1 excluded)
│   ├── wraith-core/        # Frame, session, congestion, Node API
│   ├── wraith-crypto/      # Noise, AEAD, Elligator2, ratcheting
│   ├── wraith-transport/   # AF_XDP, io_uring, UDP
│   ├── wraith-obfuscation/ # Padding, timing, mimicry
│   ├── wraith-discovery/   # DHT, relay, NAT traversal
│   ├── wraith-files/       # Chunking, integrity, transfer
│   ├── wraith-cli/         # CLI (wraith binary)
│   ├── wraith-ffi/         # C-compatible API, JNI, UniFFI
│   └── wraith-xdp/         # eBPF/XDP (Linux-only, excluded)
├── clients/                # Client applications (12 total: 9 desktop + 2 mobile + 1 server)
├── templates/              # Configuration and ROE templates (17)
│   ├── recon/              # WRAITH-Recon ROE templates (7)
│   ├── config/             # CLI and node configuration (2)
│   ├── transfer/           # Transfer profile templates (1)
│   └── integration/        # Docker Compose, systemd service (2)
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
- Rust 1.88+ (2024 Edition, MSRV: 1.88)

### Key Dependencies
- `chacha20poly1305`, `x25519-dalek`, `blake3` - Cryptography
- `snow` - Noise Protocol framework
- `io-uring` - Async file I/O (Linux)
- `tokio` - Async runtime
- `clap` - CLI parsing

### Threading Model
Thread-per-core with no locks in hot path. Sessions pinned to cores, NUMA-aware allocation.

## Implementation Status

| Crate | Status | Features |
|-------|--------|----------|
| wraith-core | ✅ Complete | Frame (SIMD), Session, Stream, BBR, Migration, Node API, Security Monitor |
| wraith-crypto | ✅ Complete | Ed25519, X25519+Elligator2, AEAD, Noise_XX, Ratchet, Noise Integration |
| wraith-transport | ✅ Complete | AF_XDP, io_uring, UDP, worker pools, NUMA-aware, buffer pools |
| wraith-obfuscation | ✅ Complete | Padding (5), Timing (5), Mimicry (TLS/WebSocket/DoH), Cover Traffic |
| wraith-discovery | ✅ Complete | Kademlia DHT, STUN, ICE (RFC 8445), relay |
| wraith-files | ✅ Complete | io_uring I/O, chunking, tree hashing, reassembly |
| wraith-cli | ✅ Complete | Full CLI with config, progress, commands |
| wraith-ffi | ✅ Complete | C-compatible API, FFI-safe types, JNI, UniFFI |
| wraith-xdp | Not started | Requires eBPF toolchain (future) |

### Client Applications

| Client | Status | Features |
|--------|--------|----------|
| wraith-transfer | ✅ Complete | Tauri desktop file transfer |
| wraith-chat | ✅ Complete | E2EE messaging, voice/video calls, groups |
| wraith-android | ✅ Complete | Kotlin + JNI, Keystore, FCM push |
| wraith-ios | ✅ Complete | Swift + UniFFI, Keychain, APNs push |
| wraith-sync | ✅ Complete | Delta sync, version history |
| wraith-share | ✅ Complete | Swarm transfers, link sharing |
| wraith-stream | ✅ Complete | AV1/VP9/H.264, adaptive bitrate |
| wraith-mesh | ✅ Complete | Topology visualization, DHT inspection |
| wraith-publish | ✅ Complete | Ed25519 signatures, RSS feeds |
| wraith-vault | ✅ Complete | Shamir SSS, erasure coding, guardians |
| wraith-recon | ✅ Complete | Packet capture, protocol analysis |
| wraith-redops | ✅ Complete | Team Server, Operator Client, Spectre Implant (isolated workspace, 11 tests run separately) |

