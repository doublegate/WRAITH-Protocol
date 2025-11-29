# Changelog

All notable changes to WRAITH Protocol will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

## [0.1.0] - Unreleased

Initial development release. See [ROADMAP](to-dos/ROADMAP.md) for planned features.

### Planned Features
- Complete protocol implementation (7 phases)
- 8 client applications across 3 priority tiers
- Cross-platform support (Linux, macOS, Windows)
- Mobile clients (Android, iOS)
- Post-quantum cryptography (hybrid mode)

---

[Unreleased]: https://github.com/doublegate/WRAITH-Protocol/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/doublegate/WRAITH-Protocol/releases/tag/v0.1.0
