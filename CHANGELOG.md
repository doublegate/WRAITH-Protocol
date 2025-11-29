# Changelog

All notable changes to WRAITH Protocol will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial Rust workspace structure with 7 crates
  - `wraith-core`: Protocol primitives, frames, sessions, congestion control
  - `wraith-crypto`: XChaCha20-Poly1305 AEAD, key ratcheting, Elligator2, Noise_XX
  - `wraith-transport`: UDP fallback, io_uring acceleration stubs
  - `wraith-obfuscation`: Padding, timing, cover traffic generation
  - `wraith-discovery`: DHT peer discovery, NAT traversal
  - `wraith-files`: File chunking, BLAKE3 hashing
  - `wraith-cli`: Command-line interface with clap
- `xtask` build automation
- GitHub Actions CI workflow
- Comprehensive documentation structure
  - Architecture documentation (5 documents)
  - Protocol sprint planning (7 phases)
  - Client application sprint planning (8 clients)
  - Project roadmap
- Development configuration (rustfmt, clippy)
- Standard repository files (LICENSE, SECURITY, CODE_OF_CONDUCT)
- GitHub issue and PR templates

### Security
- Cryptographic foundation designed for forward secrecy
- Traffic analysis resistance via Elligator2 encoding
- AEAD encryption with XChaCha20-Poly1305

## [0.1.0] - Unreleased

Initial development release. See [ROADMAP](to-dos/ROADMAP.md) for planned features.

---

[Unreleased]: https://github.com/parobek/WRAITH-Protocol/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/parobek/WRAITH-Protocol/releases/tag/v0.1.0
