# WRAITH Protocol

**W**ire-speed **R**esilient **A**uthenticated **I**nvisible **T**ransfer **H**andler

A decentralized secure file transfer protocol optimized for high-throughput, low-latency operation with strong security guarantees and traffic analysis resistance.

![WRAITH Protocol Banner](images/wraith-protocol_banner-graphic.jpg)

[![CI Status](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/ci.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/ci.yml)
[![CodeQL](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/codeql.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/codeql.yml)
[![Release](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/release.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/release.yml)
[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/doublegate/WRAITH-Protocol/releases)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Edition](https://img.shields.io/badge/edition-2024-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

## Current Status

**Version:** 0.1.0 (Initial Release)

WRAITH Protocol has completed Phase 1 Foundation development with a fully functional core protocol implementation. The v0.1.0 release delivers production-ready frame encoding, session management, stream multiplexing, and congestion control.

**Phase 1 Complete ✅ (89/789 story points, 11% overall progress)**

**Implementation Status:**
- Core workspace: 7 crates, ~3,500 lines of Rust code
- Test coverage: **110 passing tests** (104 wraith-core + 6 wraith-crypto)
  - Frame layer: 28 tests (22 unit + 6 property-based)
  - Session state: 23 tests
  - Stream multiplexing: 33 tests
  - BBR congestion: 29 tests
- Benchmarks: 19 criterion benchmarks (frame parse/build/roundtrip)
- Performance: 172M frames/sec parsing (~232 GiB/s theoretical throughput)
- Documentation: 59+ files, 40,000+ lines
- CI/CD: GitHub Actions workflows for testing, security scanning, multi-platform releases
- Security: Dependabot and CodeQL integration, weekly vulnerability scans
- Code quality: Zero warnings, all clippy lints passing

**Completed Components:**
- ✅ Frame encoding/decoding (all 12 frame types)
- ✅ Session state machine (5 states)
- ✅ Stream multiplexing (6 states, flow control)
- ✅ BBR congestion control (4 phases)
- ✅ Comprehensive test suite
- ✅ Performance benchmarks

**Next: Phase 2 - Cryptographic Layer (102 story points, 4-6 weeks)**
- Noise_XX handshake implementation
- Elligator2 encoding for traffic analysis resistance
- Symmetric and DH key ratcheting

## Features

### Performance
- **Wire-Speed Transfers**: 10+ Gbps throughput with AF_XDP kernel bypass
- **Sub-Millisecond Latency**: <1ms packet processing with io_uring
- **Zero-Copy I/O**: Direct NIC-to-application data path
- **BBR Congestion Control**: Optimal bandwidth utilization

### Security
- **Strong Encryption**: XChaCha20-Poly1305 AEAD (256-bit security)
- **Perfect Forward Secrecy**: Double ratchet key derivation
- **Mutual Authentication**: Noise_XX handshake pattern
- **Memory Safety**: Pure Rust implementation

### Privacy
- **Traffic Analysis Resistance**: Elligator2 key encoding
- **Protocol Mimicry**: TLS, WebSocket, DNS-over-HTTPS wrappers
- **Timing Obfuscation**: Configurable packet timing
- **Cover Traffic**: Constant-rate transmission mode

### Decentralization
- **Privacy-Enhanced DHT**: Anonymous peer discovery
- **NAT Traversal**: STUN-like hole punching, relay fallback
- **Connection Migration**: Seamless IP address changes
- **No Central Servers**: Fully peer-to-peer operation

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

### Security Testing
- [Security Testing Parameters](ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)
- [WRAITH-Recon Documentation](docs/clients/wraith-recon/)
- [WRAITH-RedOps Documentation](docs/clients/wraith-redops/)

## Roadmap

WRAITH Protocol development follows a structured 7-phase approach spanning 32-44 weeks:

### Protocol Development (789 Story Points)

| Phase | Focus | Duration | Story Points | Status |
|-------|-------|----------|--------------|--------|
| **Phase 1** | Foundation & Core Types | 4-6 weeks | 89 | ✅ **Complete** |
| **Phase 2** | Cryptographic Layer | 4-6 weeks | 102 | 🔄 Next |
| **Phase 3** | Transport & Kernel Bypass | 6-8 weeks | 156 | Planned |
| **Phase 4** | Obfuscation & Stealth | 3-4 weeks | 76 | Planned |
| **Phase 5** | Discovery & NAT Traversal | 5-7 weeks | 123 | Planned |
| **Phase 6** | Integration & Testing | 4-5 weeks | 98 | Planned |
| **Phase 7** | Hardening & Optimization | 6-8 weeks | 145 | Planned |

**Progress:** 89/789 story points delivered (11% complete)

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
- **AEAD:** XChaCha20-Poly1305 (256-bit security, 192-bit nonce)
- **Key Exchange:** X25519 with Elligator2 indistinguishability
- **Hash Function:** BLAKE3 (tree-parallelizable, faster than BLAKE2/SHA-3)
- **Handshake:** Noise_XX (mutual authentication, identity hiding)

### Security Features
- **Forward Secrecy:** Double ratchet with DH and symmetric ratchets
- **Traffic Analysis Resistance:** Elligator2, padding, timing obfuscation, protocol mimicry
- **Memory Safety:** Rust with no unsafe code in cryptographic paths
- **Constant-Time Operations:** Side-channel resistant implementations
- **Automated Security Scanning:** Dependabot, CodeQL, RustSec advisories

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
1. ✅ **Phase 1 Complete** - Core protocol foundation implemented (110 tests, 172M frames/sec)
2. Begin Phase 2 cryptographic layer implementation
3. Implement Noise_XX handshake with Elligator2 encoding
4. Add symmetric and DH key ratcheting
5. Maintain test coverage (current: 110 tests, target: maintain 80%+ coverage)

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

**Status:** Early Development (v0.1.0) | **License:** MIT | **Language:** Rust 2024
