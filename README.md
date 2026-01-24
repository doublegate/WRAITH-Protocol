# WRAITH Protocol

**W**ire-speed **R**esilient **A**uthenticated **I**nvisible **T**ransfer **H**andler

A decentralized secure file transfer protocol optimized for high-throughput, low-latency operation with strong security guarantees and traffic analysis resistance.

![WRAITH Protocol Banner](images/wraith-protocol_banner-graphic.jpg)

[![CI Status](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/ci.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/ci.yml)
[![CodeQL](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/codeql.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/codeql.yml)
[![Release](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/release.yml/badge.svg)](https://github.com/doublegate/WRAITH-Protocol/actions/workflows/release.yml)
[![Version](https://img.shields.io/badge/version-2.0.1-blue.svg)](https://github.com/doublegate/WRAITH-Protocol/releases)
[![Security](https://img.shields.io/badge/security-audited-green.svg)](docs/security/SECURITY_AUDIT_v1.1.0.md)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Edition](https://img.shields.io/badge/edition-2024-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

## Current Status

**Version:** 2.0.1 | **Development Phase:** Phase 24 Complete

WRAITH Protocol v2.0.0 is a major milestone release with 10 production-ready desktop/mobile applications. Version 2.0.0 delivers Phase 24 (WRAITH-Vault), a distributed secret storage application implementing Shamir's Secret Sharing with threshold cryptography, guardian-based key distribution, erasure coding for redundancy, and scheduled automatic backups.

**Project Metrics (2026-01-24):**
- **Code Volume:** ~87,000 lines of Rust code across protocol crates + ~30,000 lines in client applications (Kotlin/Swift/TypeScript)
- **Test Coverage:** 1,993 tests passing (4 ignored) - 100% pass rate
- **Documentation:** 130+ markdown files, ~90,000+ lines of comprehensive documentation
- **Dependencies:** 295 audited packages (zero vulnerabilities via cargo-audit)
- **Security:** Grade A+ (EXCELLENT), zero vulnerabilities, comprehensive DPI evasion validation
- **Quality:** Code quality 98/100, zero compiler/clippy warnings, 3.5% technical debt ratio, production-ready codebase
- **Client Applications:** 10 production-ready applications (4 Tier 1 + 3 Tier 2 + 3 Tier 3) with full protocol integration
- **CI/CD:** GitHub Actions updated (upload-artifact v6, download-artifact v7, cache v5)

**v2.0.0 Highlights (Phase 24 Complete):**
- **WRAITH-Vault Complete**: New Tier 3 distributed secret storage application (Phase 24)
- **Shamir's Secret Sharing**: Threshold cryptography with configurable k-of-n splitting
- **Guardian System**: Trust-based key fragment distribution with device tracking
- **Erasure Coding**: Reed-Solomon redundancy for data protection (configurable data/parity shards)
- **Encrypted Backup**: AES-256-GCM encrypted incremental backups with zstd compression
- **Scheduled Automation**: Cron-based automatic backup scheduling
- **Recovery System**: Guardian-based secret reconstruction with progress tracking
- **Content Deduplication**: BLAKE3-based chunk deduplication for storage efficiency
- **SQLite Database**: Secure local storage with encryption-at-rest
- **99 Tests**: Comprehensive test coverage across all modules

**Phase 24 Highlights (WRAITH-Vault):**
- **Secret Management**: Create, store, retrieve, and delete encrypted secrets
- **Shamir Splitting**: k-of-n threshold scheme (1 ≤ k ≤ n ≤ 255)
- **Guardian Network**: Trust levels, device management, share distribution
- **Erasure Coding**: Reed-Solomon with automatic repair capability
- **Backup System**: Incremental snapshots with manifest tracking
- **Recovery Flow**: Multi-guardian share collection with verification
- **Deduplication**: Content-defined chunking with BLAKE3 fingerprints
- **Progress Tracking**: Real-time backup/restore progress reporting
- **Tauri 2.0 Desktop**: React 18 + TypeScript frontend with dark theme
- **99 Tests**: Full coverage including Shamir, erasure, backup, and recovery

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

**Client Applications (Phase 15-17):**

**WRAITH-Transfer (Desktop):**
- Cross-platform GUI (Windows, macOS, Linux with X11/Wayland support)
- Tauri 2.0 backend with full wraith-core integration
- React 18 + TypeScript frontend with Vite bundling
- Tailwind CSS v4 with WRAITH brand colors (#FF5722 primary, #4A148C secondary)
- Intuitive file transfer interface with drag-and-drop support
- Real-time session and transfer monitoring
- Thread-safe state management (Arc<RwLock<Node>>)
- 10 IPC commands for node/session/transfer control
- Zustand state management (nodeStore, transferStore, sessionStore)
- Production-ready packaging for all platforms
- v1.5.8: Wayland compatibility fix (resolves KDE Plasma 6 crashes)

**WRAITH-Android (Mobile) - Phase 17 Enhanced:**
- Native Android application with Kotlin/Jetpack Compose Material Design 3 UI
- JNI bindings to wraith-ffi for full protocol integration (Phase 17: actual WRAITH protocol)
- Multi-architecture support (arm64, arm, x86_64, x86) via cargo-ndk
- Background foreground service for continuous transfers
- Storage permissions handling for Android 8.0+
- Coroutine-based async operations with high-level Kotlin API
- ProGuard/R8 optimization for production builds
- **Phase 17 Additions:**
  - Android Keystore integration for hardware-backed secure key storage
  - DHT peer discovery with mobile network optimization
  - NAT traversal with mobile-aware connection handling
  - Firebase Cloud Messaging (FCM) for push notifications
- ~3,800 lines (1,200 Rust, 2,400 Kotlin, 200 Gradle) | 96 tests

**WRAITH-iOS (Mobile) - Phase 17 Enhanced:**
- Native iOS application with SwiftUI (iOS 16.0+)
- UniFFI bindings for automatic Swift interface generation (Phase 17: actual WRAITH protocol)
- Tab-based navigation (Home, Transfers, Sessions, Settings)
- MVVM architecture with ObservableObject state management
- Swift Package Manager integration
- Background task support for iOS lifecycle management
- Native iOS design patterns and accessibility features
- **Phase 17 Additions:**
  - iOS Keychain integration for secure key storage
  - DHT peer discovery with mobile network optimization
  - NAT traversal with cellular/WiFi handoff support
  - Apple Push Notification Service (APNs) integration
- ~2,650 lines (750 Rust UniFFI, 1,900 Swift) | 93 tests

**WRAITH-Chat (Desktop Messaging) - Phase 17 Enhanced:**
- Secure E2EE messaging application (Tauri 2.0 + React 18)
- Signal Protocol Double Ratchet implementation (forward secrecy + post-compromise security)
- SQLCipher encrypted database (AES-256, PBKDF2-HMAC-SHA512, 64,000 iterations)
- X25519 DH key exchange with Elligator2 encoding
- ChaCha20-Poly1305 AEAD encryption with 192-bit nonce
- Out-of-order message handling with skipped key storage (max 1,000)
- Contact management with safety number verification (SHA-256)
- React frontend with Zustand state management and dark theme
- Real-time message synchronization with infinite scroll
- **Phase 17 Additions:**
  - Voice Calling: Opus codec (48kHz), RNNoise noise suppression, WebRTC echo cancellation
  - Video Calling: VP8/VP9 codecs, adaptive bitrate (360p-1080p), jitter buffer
  - Group Messaging: Sender Keys protocol for O(1) encryption efficiency
  - 16 voice IPC commands (call initiation, media control, call state)
  - 16 video IPC commands (camera control, quality settings, screen sharing)
  - 11 group IPC commands (create, manage members, admin controls)
- ~5,200 lines (2,800 Rust backend, 2,400 TypeScript/React frontend)
- 49 IPC commands total (10 messaging + 16 voice + 16 video + 11 group)

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

**Note:** WRAITH Protocol v1.7.0 features a complete Node API and protocol implementation with fully integrated CLI commands. The wraith-cli binary provides production-ready command-line access to all protocol features including `ping` for connectivity testing and `config` for runtime configuration.

```bash
# Generate identity keypair
wraith keygen --output ~/.wraith/identity.key

# Send a file to peer
wraith send document.pdf <peer-id>

# Receive files
wraith receive --output ./downloads

# Run as background daemon
wraith daemon --bind 0.0.0.0:0

# Check node status
wraith status

# List discovered peers and sessions
wraith peers

# Manage configuration
wraith config --show
```

For detailed usage, see [User Guide](docs/USER_GUIDE.md) and [Tutorial](docs/TUTORIAL.md).

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
│   ├── wraith-ffi/             # Foreign function interface (C/JNI bindings)
│   └── wraith-xdp/             # eBPF/XDP programs (Linux-only, excluded)
├── clients/                     # Client applications
│   ├── wraith-transfer/        # Desktop file transfer (Tauri 2.0 + React 18)
│   ├── wraith-android/         # Android mobile client (Kotlin + Jetpack Compose)
│   ├── wraith-ios/             # iOS mobile client (Swift + SwiftUI)
│   ├── wraith-chat/            # E2EE messaging (Tauri 2.0 + React 18 + SQLCipher)
│   ├── wraith-sync/            # File synchronization (Tauri 2.0 + React 18)
│   ├── wraith-share/           # Distributed file sharing (Tauri 2.0 + React 18)
│   ├── wraith-publish/         # Decentralized content publishing (Tauri 2.0 + React 18)
│   └── wraith-vault/           # Distributed secret storage (Tauri 2.0 + React 18)
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
├── tests/                       # Integration tests and benchmarks
└── xtask/                       # Build automation
```

### Crate Overview

### Protocol Crates

| Crate | Description | Tests | Status |
|-------|-------------|-------|--------|
| **wraith-core** | Frame parsing (SIMD), sessions, congestion control, ring buffers, Node API | 414 | ✅ Complete |
| **wraith-crypto** | Ed25519, X25519+Elligator2, AEAD, Noise_XX, Double Ratchet | 127 | ✅ Complete |
| **wraith-transport** | AF_XDP, io_uring, UDP sockets, worker pools, NUMA-aware allocation | 130 | ✅ Complete |
| **wraith-obfuscation** | Padding (5 modes), timing (5 distributions), protocol mimicry (TLS/WS/DoH) | 111 | ✅ Complete |
| **wraith-discovery** | Kademlia DHT, STUN, ICE, relay infrastructure | 231 | ✅ Complete |
| **wraith-files** | File chunking, BLAKE3 tree hashing, reassembly with io_uring | 34 | ✅ Complete |
| **wraith-cli** | Command-line interface with full Node API integration | 8 | ✅ Complete |
| **wraith-ffi** | Foreign function interface (C-compatible API, JNI bindings) | 6 | ✅ Complete |
| **wraith-xdp** | eBPF/XDP programs (requires eBPF toolchain, excluded from default build) | 0 | 📋 Planned |

**Total Protocol:** 1,061 tests across 8 active crates

### Client Applications

| Application | Description | Platform | Tests | Status |
|-------------|-------------|----------|-------|--------|
| **wraith-transfer** | P2P file transfer with drag-and-drop GUI (Tauri 2.0 + React 18 + TypeScript) | Desktop | 68 | ✅ v1.7.0 |
| **wraith-android** | Mobile client with full protocol integration (Kotlin + Jetpack Compose + JNI) | Android | 96 | ✅ v1.7.0 |
| **wraith-ios** | Mobile client with full protocol integration (Swift + SwiftUI + UniFFI) | iOS | 103 | ✅ v1.7.0 |
| **wraith-chat** | E2EE messaging with voice/video/groups (Tauri 2.0 + React 18 + SQLCipher) | Desktop | 76 | ✅ v1.7.0 |
| **wraith-sync** | Decentralized file sync with delta transfers and version history (Tauri 2.0 + React 18) | Desktop | 17 | ✅ v1.7.0 |
| **wraith-share** | Distributed anonymous file sharing with swarm transfers and link sharing (Tauri 2.0 + React 18) | Desktop | 24 | ✅ v1.8.0 |
| **wraith-stream** | Secure media streaming with AV1/VP9/H.264, adaptive bitrate, live/VOD (Tauri 2.0 + React 18) | Desktop | 27 | ✅ v1.8.5 |
| **wraith-mesh** | IoT mesh networking with topology visualization, DHT inspection, diagnostics (Tauri 2.0 + React 18) | Desktop | 21 | ✅ v1.9.0 |
| **wraith-publish** | Decentralized content publishing with Ed25519 signatures, RSS feeds, DHT distribution (Tauri 2.0 + React 18) | Desktop | 56 | ✅ v1.9.5 |
| **wraith-vault** | Distributed secret storage with Shamir's Secret Sharing, guardian system, erasure coding (Tauri 2.0 + React 18) | Desktop | 99 | ✅ v2.0.0 |

**Total Clients:** 587 tests, 10 production applications (~37,000 lines: TypeScript/Kotlin/Swift)

### Integration Tests & Benchmarks

| Component | Tests | Purpose |
|-----------|-------|---------|
| **Integration Tests** | 323 | Cross-crate protocol integration and end-to-end scenarios |
| **Benchmarks** | - | Performance validation (frame parsing, AEAD, hashing, file operations) |

**Project Total:** 1,993 tests (4 ignored) - 100% pass rate

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
- [UI/UX Design Reference](docs/clients/UI-UX-DESIGN-REFERENCE.md) - Comprehensive design system for all WRAITH clients
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
| **WRAITH-Transfer** | Desktop P2P file transfer with drag-and-drop GUI (Tauri 2.0 + React 18) | ✅ **Complete (v1.5.0)** | 102 |
| **WRAITH-Android** | Android mobile client with Kotlin/Jetpack Compose (Material Design 3, JNI bindings) | ✅ **Complete (v1.6.0)** | ~60 |
| **WRAITH-iOS** | iOS mobile client with Swift/SwiftUI (UniFFI bindings, iOS 16.0+) | ✅ **Complete (v1.6.0)** | ~60 |
| **WRAITH-Chat** | E2EE messaging with Signal Protocol Double Ratchet (Tauri 2.0 + SQLCipher) | ✅ **Complete (v1.6.0)** | 182 |

### Tier 2: Specialized Applications (Medium Priority)

| Client | Description | Status | Story Points |
|--------|-------------|--------|--------------|
| **WRAITH-Sync** | Decentralized backup synchronization with delta sync and version history | ✅ **Complete (v1.7.0)** | 136 |
| **WRAITH-Share** | Distributed anonymous file sharing (BitTorrent-like) with swarm transfers and link sharing | ✅ **Complete (v1.8.0)** | 123 |
| **WRAITH-Stream** | Secure media streaming with live/VOD, adaptive bitrate, encrypted segments | ✅ **Complete (v1.8.5)** | 71 |

### Tier 3: Advanced Applications (Lower Priority)

| Client | Description | Status | Story Points |
|--------|-------------|--------|--------------|
| **WRAITH-Mesh** | IoT mesh networking with topology visualization and diagnostics | ✅ **Complete (v1.9.0)** | 60 |
| **WRAITH-Publish** | Censorship-resistant publishing platform (blogs, wikis) | ✅ **Complete (v1.9.5)** | 76 |
| **WRAITH-Vault** | Distributed secret storage using Shamir Secret Sharing | ✅ **Complete (v2.0.0)** | 94 |

### Tier 3: Security Testing (Specialized - Authorized Use Only)

| Client | Description | Status | Story Points |
|--------|-------------|--------|--------------|
| **WRAITH-Recon** | Network reconnaissance & data exfiltration assessment | Planned | 55 |
| **WRAITH-RedOps** | Red team operations platform with C2 infrastructure | Planned | 89 |

**Total Ecosystem:** 10 clients, ~1,148 story points (10 complete)

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
- ✅ Phase 15: WRAITH-Transfer Desktop Application (102 SP)
- ✅ Phase 16: Mobile Clients & WRAITH-Chat (302 SP)
  - WRAITH-Android: Native Android client with Kotlin/Jetpack Compose, JNI bindings (~60 SP)
  - WRAITH-iOS: Native iOS client with Swift/SwiftUI, UniFFI bindings (~60 SP)
  - WRAITH-Chat: E2EE messaging with Double Ratchet, SQLCipher database (182 SP)
- ✅ Phase 17: Full Mobile Integration & Real-Time Communications (320 SP)
  - Mobile Protocol Integration: Android JNI and iOS UniFFI with actual WRAITH protocol (26 tests)
  - Mobile Secure Storage: Android Keystore and iOS Keychain integration (45 tests)
  - Mobile Discovery: DHT and NAT traversal for mobile networks (63 tests)
  - Push Notifications: FCM (Android) and APNs (iOS) (107 tests)
  - Voice Calling: Opus codec, RNNoise, echo cancellation
  - Video Calling: VP8/VP9, adaptive bitrate (38 tests)
  - Group Messaging: Sender Keys protocol for O(1) encryption
  - Integration Testing: 260 end-to-end tests
- ✅ Phase 18: Chat Protocol Wire-Up (45 SP)
  - Voice call signaling via WRAITH protocol streams
  - Video call coordination with quality negotiation
  - Group messaging protocol integration with Sender Keys distribution
- ✅ Phase 19: DNS STUN Resolution (35 SP)
  - Dynamic STUN server discovery via DNS SRV records
  - Health monitoring with automatic failover
  - TTL-based caching for performance optimization
- ✅ Phase 20: WRAITH-Share Complete (123 SP)
  - Distributed anonymous file sharing application
  - Swarm-based multi-peer transfers
  - Link-based sharing with access control

**Total Development:** 2,685 story points delivered across 24 phases

**Upcoming:**
- 📋 Post-quantum cryptography (Kyber/Dilithium hybrid mode)
- 📋 Formal verification of cryptographic protocols
- 📋 XDP/eBPF kernel bypass implementation
- 📋 Security testing tools (WRAITH-Recon, WRAITH-RedOps)

See [ROADMAP.md](to-dos/ROADMAP.md) and [Protocol Development History](docs/archive/README_Protocol-DEV.md) for detailed planning and phase accomplishments.

### Client Applications

10 client applications across 3 priority tiers:
- **Tier 1 (Complete):**
  - ✅ WRAITH-Transfer (Desktop P2P file transfer - v1.5.0)
  - ✅ WRAITH-Android (Android mobile client with full protocol integration - v1.7.0)
  - ✅ WRAITH-iOS (iOS mobile client with full protocol integration - v1.7.0)
  - ✅ WRAITH-Chat (E2EE messaging with voice/video/groups - v1.7.0)
- **Tier 2 (Complete):**
  - ✅ WRAITH-Sync (file synchronization - v1.7.0)
  - ✅ WRAITH-Share (distributed file sharing - v1.8.0)
  - ✅ WRAITH-Stream (secure media streaming - v1.8.5)
- **Tier 3 (Complete):**
  - ✅ WRAITH-Mesh (IoT mesh networking - v1.9.0)
  - ✅ WRAITH-Publish (decentralized publishing - v1.9.5)
  - ✅ WRAITH-Vault (distributed secret storage - v2.0.0)
- **Security Testing (Planned):** WRAITH-Recon, WRAITH-RedOps (authorized use only)

**Status:** 10 of 10 core clients complete (1,004 SP delivered)

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
- **Path Filtering:** Skips CI runs for documentation-only changes

### Workflow Optimizations (v2.0.1)
- **Reusable Setup Workflow** (`setup.yml`): Common environment setup across all workflows
  - Cross-platform support (Ubuntu, macOS, Windows)
  - Rust toolchain with configurable components
  - Node.js and Tauri CLI installation
  - SQLCipher support for all platforms
- **Optimized Job Dependencies:** Fast failure detection with early gates
  - `fmt` runs immediately (no dependencies)
  - `check` runs before expensive test matrix
  - `clippy` reuses build artifacts from `check`
- **Client Exclusions:** 8 Tauri clients excluded from protocol-level CI
- **Updated GitHub Actions:** checkout@v6, cache@v5, upload-artifact@v6, download-artifact@v7

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
- **Client Builds:** Tauri desktop application packaging for all platforms

See [CI Workflow](.github/workflows/ci.yml), [Setup Workflow](.github/workflows/setup.yml), [CodeQL Workflow](.github/workflows/codeql.yml), [Fuzz Workflow](.github/workflows/fuzz.yml), and [Release Workflow](.github/workflows/release.yml) for configuration details.

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
- **Test Coverage:** 1,993 tests (4 ignored) covering all protocol layers
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

**Status:** v2.0.1 Phase 24 Complete | **License:** MIT | **Language:** Rust 2024 (MSRV 1.85) | **Tests:** 1,993 passing (4 ignored) | **Quality:** Production-ready, 0 vulnerabilities, zero warnings, 98/100 quality grade | **Clients:** 10 applications with full protocol integration, voice/video calling, group messaging, file synchronization, distributed file sharing, secure media streaming, IoT mesh networking, decentralized publishing, and distributed secret storage

*Last Updated: 2026-01-24*
