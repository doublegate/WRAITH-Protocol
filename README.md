# WRAITH Protocol

**W**ire-speed **R**esilient **A**uthenticated **I**nvisible **T**ransfer **H**andler

A decentralized secure file transfer protocol optimized for high-throughput, low-latency operation with strong security guarantees and traffic analysis resistance.

![WRAITH Protocol Banner](images/wraith-protocol_banner-graphic.jpg)

[![CI Status](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/ci.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/ci.yml)
[![CodeQL](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/codeql.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/codeql.yml)
[![Release](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/release.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/release.yml)
[![Version](https://img.shields.io/badge/version-1.5.5-blue.svg)](https://github.com/doublegate/WRAITH-Protocol/releases)
[![Security](https://img.shields.io/badge/security-audited-green.svg)](docs/security/SECURITY_AUDIT_v1.1.0.md)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Edition](https://img.shields.io/badge/edition-2024-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

## Current Status

**Version:** 1.5.5 Technical Debt & Quality Release | **Development Phase:** Phase 15 Complete

WRAITH Protocol is production-ready with a cross-platform desktop application. Phase 15 delivers WRAITH Transfer, a Tauri 2.0-based desktop client with React 18 frontend, providing full wraith-core integration with an intuitive UI for secure file transfers.

**Project Metrics (2025-12-08):**
- **Code Volume:** ~41,177 lines of Rust code (~30,876 LOC + 2,743 comments + 7,558 blanks) across 115 source files
- **Test Coverage:** 1,303 total tests (1,280 passing, 23 ignored) - 100% pass rate on active tests
- **Documentation:** 100+ markdown files, ~46,000+ lines of comprehensive documentation
- **Dependencies:** 286 audited packages (zero vulnerabilities via cargo-audit)
- **Security:** Grade A+ (EXCELLENT), zero vulnerabilities, comprehensive DPI evasion validation
- **Quality:** Code quality 98/100, zero compiler/clippy warnings, 3.8% technical debt ratio, production-ready codebase

For detailed development history and phase accomplishments, see [Protocol Development History](docs/archive/README_Protocol-DEV.md).

## Features

### Core Capabilities

**High-Performance Transport:**
- Wire-speed transfers (10+ Gbps with AF_XDP kernel bypass)
- Sub-millisecond latency (<1ms packet processing with io_uring)
- Zero-copy I/O via AF_XDP UMEM and io_uring registered buffers
- BBR congestion control with optimal bandwidth utilization
- Async file I/O with io_uring

**Security & Privacy:**
- End-to-end encryption (XChaCha20-Poly1305 AEAD)
- Perfect forward secrecy (Double Ratchet with DH and symmetric ratcheting)
- Mutual authentication (Noise_XX handshake pattern)
- Ed25519 digital signatures for identity verification
- BLAKE3 cryptographic hashing
- Traffic analysis resistance (Elligator2 key encoding)
- Replay protection (64-bit sliding window)
- Key commitment for AEAD (prevents multi-key attacks)

**Traffic Obfuscation:**
- Packet padding (5 modes: PowerOfTwo, SizeClasses, ConstantRate, Statistical)
- Timing obfuscation (5 distributions: Fixed, Uniform, Normal, Exponential)
- Protocol mimicry (TLS 1.3, WebSocket, DNS-over-HTTPS)
- Cover traffic generation (Constant, Poisson, Uniform distributions)
- Adaptive threat-level profiles (Low, Medium, High, Paranoid)

**Decentralized Discovery:**
- Privacy-enhanced Kademlia DHT with BLAKE3 NodeIds
- S/Kademlia Sybil resistance (crypto puzzle-based NodeId generation)
- NAT traversal (STUN client, ICE-lite UDP hole punching)
- DERP-style relay infrastructure with automatic fallback
- Connection migration with PATH_CHALLENGE/PATH_RESPONSE

**File Transfer:**
- Chunked file transfer with BLAKE3 tree hashing
- Multi-peer downloads with parallel chunk fetching
- Resume support with missing chunks detection
- Real-time progress tracking (bytes, speed, ETA)
- Chunk verification (<1μs per chunk)

**Node API:**
- High-level protocol orchestration layer
- Lifecycle management (start/stop)
- Session management (Noise_XX handshake)
- File transfer coordination
- DHT integration (peer discovery, announcements)
- NAT traversal integration
- Health monitoring with failed ping detection
- Connection migration (PATH_CHALLENGE/PATH_RESPONSE)
- Lock-free ring buffers (SPSC/MPSC) for packet processing
- Comprehensive configuration system (6 subsystems)

**WRAITH Transfer Desktop Application:**
- Cross-platform GUI (Windows, macOS, Linux)
- Tauri 2.0 backend with full wraith-core integration
- React 18 + TypeScript frontend with Vite bundling
- Tailwind CSS v4 with WRAITH brand colors (#FF5722 primary, #4A148C secondary)
- Intuitive file transfer interface with drag-and-drop support
- Real-time session and transfer monitoring
- Thread-safe state management (Arc<RwLock<Node>>)
- 10 IPC commands for node/session/transfer control
- Zustand state management (nodeStore, transferStore, sessionStore)
- Production-ready packaging for all platforms

![WRAITH Protocol Architecture](images/wraith-protocol_arch-infographic.jpg)

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

**Note:** WRAITH Protocol v1.3.0 has a complete Node API and protocol implementation. The CLI interface is scaffolded but not yet fully integrated with the Node API. The following commands represent the planned interface:

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

For current development status, see [ROADMAP.md](to-dos/ROADMAP.md).

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
│   ├── archive/                # Archived documentation and development history
│   ├── architecture/           # Protocol design (5 docs)
│   ├── engineering/            # Development guides (4 docs)
│   ├── integration/            # Embedding & platform support (3 docs)
│   ├── testing/                # Testing strategies (3 docs)
│   ├── operations/             # Deployment & monitoring (3 docs)
│   └── clients/                # Client application docs (37 docs)
├── to-dos/                      # Sprint planning
│   ├── protocol/               # Implementation phases
│   ├── clients/                # Client application sprints
│   ├── ROADMAP.md              # Project roadmap
│   └── ROADMAP-clients.md      # Client roadmap
├── ref-docs/                    # Technical specifications
└── xtask/                       # Build automation
```

### Crate Overview

| Crate | Description | LOC | Code | Comments | Tests |
|-------|-------------|-----|------|----------|-------|
| **wraith-core** | Frame parsing, sessions, congestion, ring buffers, Node API | ~4,800+ | - | - | 263 (6 ignored) |
| **wraith-crypto** | Ed25519, X25519, Elligator2, AEAD, Noise_XX, Double Ratchet | ~2,500+ | - | - | 125 |
| **wraith-discovery** | Kademlia DHT, STUN, ICE, relay | ~3,500+ | - | - | 15 |
| **wraith-transport** | AF_XDP, io_uring, UDP, worker pools | ~2,800+ | - | - | 33 |
| **wraith-obfuscation** | Padding, timing, protocol mimicry | ~3,500+ | - | - | 154 |
| **wraith-files** | File chunking, tree hashing, reassembly | ~1,300 | - | - | 24 |
| **wraith-cli** | Command-line interface | ~1,100 | - | - | 0 |
| **wraith-ffi** | Foreign function interface for C/C++ integration | ~1,200 | - | - | 7 |
| **wraith-transfer** | Tauri 2.0 desktop application (Rust + React + TypeScript) | ~12,500 | - | - | 0 |
| **wraith-xdp** | eBPF/XDP programs (excluded from default build) | 0 | 0 | 0 | 0 |

**Total Protocol:** ~41,177 lines (~30,876 LOC + 2,743 comments + 7,558 blanks) across 115 Rust files, 1,303 tests (1,280 passing, 23 ignored)

## Documentation

### Getting Started
- [User Guide](docs/USER_GUIDE.md) - Installation, quick start, CLI reference
- [Configuration Reference](docs/CONFIG_REFERENCE.md) - Complete TOML configuration
- [Tutorial](docs/TUTORIAL.md) - Step-by-step getting started guide with practical examples
- [Troubleshooting](docs/TROUBLESHOOTING.md) - Common issues and solutions

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
- [Protocol Development History](docs/archive/README_Protocol-DEV.md) - Detailed phase-by-phase development timeline
- [Client Applications Development History](docs/archive/README_Clients-DEV.md) - Client ecosystem development planning and progress

### Integration
- [Embedding Guide](docs/integration/embedding-guide.md)
- [Integration Guide](docs/INTEGRATION_GUIDE.md) - Complete library integration guide with API examples
- [Platform Support](docs/integration/platform-support.md)
- [Interoperability](docs/integration/interoperability.md)

### Security
- [Security Audit Report](docs/SECURITY_AUDIT.md) - Comprehensive security validation and recommendations
- [DPI Evasion Report](docs/security/DPI_EVASION_REPORT.md) - Deep packet inspection validation and analysis
- [Security Policy](SECURITY.md) - Vulnerability reporting and responsible disclosure

### Comparisons
- [Protocol Comparison](docs/COMPARISON.md) - WRAITH vs QUIC, WireGuard, Noise Protocol, BitTorrent

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
- [Reference Client Design](docs/clients/REFERENCE_CLIENT.md) - GUI design guidelines for client applications
- [Client Roadmap](to-dos/ROADMAP-clients.md)

### Project Planning
- [Project Roadmap](to-dos/ROADMAP.md)
- [Client Roadmap](to-dos/ROADMAP-clients.md)
- [Documentation Status](docs/DOCUMENTATION_STATUS.md)

### Technical Debt & Quality
- [Technical Debt Analysis](docs/technical/technical-debt-analysis.md) - Comprehensive code quality assessment
- [Technical Debt Action Plan](docs/technical/technical-debt-action-plan.md) - Prioritized remediation strategy
- [Technical Debt TODO List](docs/technical/technical-debt-todo-list.md) - Actionable tracking checklist

## Client Applications

WRAITH Protocol powers a comprehensive ecosystem of secure applications across 3 priority tiers:

### Tier 1: Core Applications (High Priority)

| Client | Description | Status | Story Points |
|--------|-------------|--------|--------------|
| **WRAITH-Transfer** | Direct P2P file transfer with drag-and-drop GUI | ✅ **Complete (v1.5.0)** | 102 |
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

**Total Ecosystem:** 10 clients, 1,028 story points

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

# Run benchmarks
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

## Roadmap

WRAITH Protocol development follows a structured multi-phase approach:

### Protocol Development

**Completed Phases:**
- ✅ Phase 1: Foundation & Core Types (89 SP)
- ✅ Phase 2: Cryptographic Layer (102 SP)
- ✅ Phase 3: Transport & Kernel Bypass (156 SP)
- ✅ Phase 4: Obfuscation & Stealth (243 SP)
- ✅ Phase 5: Discovery & NAT Traversal (123 SP)
- ✅ Phase 6: Integration & Testing (98 SP)
- ✅ Phase 7: Hardening & Optimization (158 SP)
- ✅ Phase 9: Node API & Protocol Orchestration (85 SP)
- ✅ Phase 10: Protocol Component Wiring (130 SP)
- ✅ Phase 11: Production Readiness (92 SP)
- ✅ Phase 12: Technical Excellence & Production Hardening (126 SP)
- ✅ Phase 13: Performance Optimization & DPI Validation (76 SP)
- ✅ Phase 14: Node API Integration & Code Quality (55 SP)
- ✅ Phase 15: WRAITH Transfer Desktop Application (102 SP)

**Total Development:** 1,635 story points delivered across 15 phases

**Upcoming:**
- 📋 Phase 16: XDP Implementation & Advanced Testing
- 📋 Phase 17+: Post-quantum cryptography, formal verification
- 📋 Client Applications (926 SP across 9 remaining applications)

See [ROADMAP.md](to-dos/ROADMAP.md) and [Protocol Development History](docs/archive/README_Protocol-DEV.md) for detailed planning and phase accomplishments.

### Client Applications

10 client applications across 3 priority tiers, including:
- **Tier 1:** WRAITH-Transfer (P2P file transfer), WRAITH-Chat (E2EE messaging)
- **Tier 2:** WRAITH-Sync (backup sync), WRAITH-Share (distributed sharing)
- **Tier 3:** WRAITH-Stream, WRAITH-Mesh, WRAITH-Publish, WRAITH-Vault
- **Security Testing:** WRAITH-Recon, WRAITH-RedOps (authorized use only)

See [Client Roadmap](to-dos/ROADMAP-clients.md) for detailed planning.

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Throughput (10 GbE) | >9 Gbps | AF_XDP with zero-copy |
| Throughput (1 GbE) | >950 Mbps | With encryption |
| Handshake Latency | <50 ms | LAN conditions |
| Packet Latency | <1 ms | NIC to application |
| Memory per Session | <10 MB | Including buffers |
| CPU @ 10 Gbps | <50% | 8-core system |

**Measured Performance (Phase 13 benchmarks):**
- **Ring Buffers:** ~100M ops/sec (SPSC), ~20M ops/sec (MPSC with 4 producers)
- **Frame Parsing:** 172M frames/sec with SIMD acceleration (AVX2/SSE4.2/NEON)
- **AEAD Encryption:** 3.2 GB/s (XChaCha20-Poly1305)
- **BLAKE3 Hashing:** 8.5 GB/s with parallelization
- **File Chunking:** 14.85 GiB/s
- **Tree Hashing:** 4.71 GiB/s in-memory, 3.78 GiB/s from disk
- **Chunk Verification:** 4.78 GiB/s
- **File Reassembly:** 5.42 GiB/s

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
- **Fuzzing:** 5 libFuzzer targets with weekly automated runs
- **Weekly Scans:** Automated security checks every Monday

### Release Automation
- **Multi-Platform Builds:** 6 platform targets (Linux x86_64/aarch64, macOS Intel/ARM, Windows)
- **Artifact Generation:** Automated binary builds with SHA256 checksums
- **GitHub Releases:** Automatic release creation from version tags
- **Changelog Integration:** Automated release notes from CHANGELOG.md

See [CI Workflow](.github/workflows/ci.yml), [CodeQL Workflow](.github/workflows/codeql.yml), [Fuzz Workflow](.github/workflows/fuzz.yml), and [Release Workflow](.github/workflows/release.yml) for configuration details.

## Security

WRAITH Protocol is designed with security as a core principle:

### Cryptographic Suite

| Function | Algorithm | Security Level | Features |
|----------|-----------|----------------|----------|
| **Signatures** | Ed25519 | 128-bit | Identity verification, ZeroizeOnDrop |
| **Key Exchange** | X25519 | 128-bit | ECDH on Curve25519 |
| **Key Encoding** | Elligator2 | Traffic analysis resistant | Indistinguishable from random |
| **AEAD** | XChaCha20-Poly1305 | 256-bit key, 192-bit nonce | Key-committing, constant-time |
| **Hash** | BLAKE3 | 128-bit collision resistance | Tree-parallelizable |
| **KDF** | HKDF-BLAKE3 | 128-bit | Context-separated key derivation |
| **Handshake** | Noise_XX_25519_ChaChaPoly_BLAKE2s | Mutual auth | Identity hiding, forward secrecy |
| **Ratcheting** | Double Ratchet | Forward & post-compromise security | Symmetric + DH ratchets |
| **Replay Protection** | 64-bit sliding window | DoS resistant | Constant-time operations |

### Security Features

**Cryptographic Guarantees:**
- **Forward Secrecy:** Double Ratchet with independent symmetric and DH ratchets
- **Post-Compromise Security:** DH ratchet heals from key compromise
- **Replay Protection:** 64-bit sliding window bitmap with constant-time operations
- **Key Commitment:** BLAKE3-based AEAD key commitment prevents multi-key attacks
- **Automatic Rekey:** Time-based, packet-count-based, byte-count-based triggers

**Traffic Analysis Resistance:**
- **Elligator2 Key Encoding:** X25519 public keys indistinguishable from random
- **Cover Traffic Generation:** Constant, Poisson, and uniform distribution modes
- **Padding:** Configurable padding modes for traffic shape obfuscation
- **Protocol Mimicry:** TLS, WebSocket, DNS-over-HTTPS wrappers

**Implementation Security:**
- **Memory Safety:** Rust with zero unsafe code in cryptographic paths
- **ZeroizeOnDrop:** Automatic zeroization of all secret key material
- **Constant-Time Operations:** Side-channel resistant implementations
- **SIMD Acceleration:** SSE2/NEON optimized with security validation
- **Unsafe Code Audit:** 100% documentation coverage with SAFETY comments

**Validation:**
- **Test Coverage:** 923 tests (913 passing, 10 ignored) covering all protocol layers
- **DPI Evasion:** Comprehensive validation against Wireshark, Zeek, Suricata, nDPI (see [DPI Evasion Report](docs/security/DPI_EVASION_REPORT.md))
- **Fuzzing:** 5 libFuzzer targets continuously testing robustness
- **Property-Based Tests:** QuickCheck-style invariant validation
- **Security Scanning:** Dependabot, CodeQL, RustSec advisories, weekly scans

### Reporting Vulnerabilities

For security issues, please see [SECURITY.md](SECURITY.md) for our security policy and responsible disclosure process.

## Getting Involved

WRAITH Protocol is in active development and we welcome contributions of all kinds:

### For Developers
- **Protocol Implementation:** Help complete advanced features and optimizations
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

See [ROADMAP.md](to-dos/ROADMAP.md) for current focus areas and planned work.

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

**Status:** v1.5.5 Technical Debt & Quality Release (Phase 15 Complete) | **License:** MIT | **Language:** Rust 2024 (MSRV 1.85) | **Tests:** 1,303 (1,280 passing + 23 ignored) | **Quality:** Production-ready, 0 vulnerabilities, zero warnings, 98/100 quality grade
