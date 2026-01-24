# WRAITH Protocol v2 Changelog Template

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Template Document
**Authors:** WRAITH Protocol Team

---

## Overview

This document provides changelog templates and guidelines for maintaining consistent release documentation across WRAITH Protocol v2 releases.

### Changelog Philosophy

Based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) principles:

1. **For Humans:** Changelogs are for humans, not machines
2. **Easy Navigation:** Every version has its own section
3. **Latest First:** Recent versions appear at the top
4. **Categorized:** Changes grouped by type
5. **Linkable:** Each version links to its diff

---

## Template

### Main Changelog Template

```markdown
# Changelog

All notable changes to WRAITH Protocol will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- New features that have been added

### Changed
- Changes in existing functionality

### Deprecated
- Features that will be removed in upcoming releases

### Removed
- Features that have been removed

### Fixed
- Bug fixes

### Security
- Security vulnerability fixes

## [2.1.0] - 2026-XX-XX

### Added
- Group communication support using TreeKEM
- Forward Error Correction (FEC) for lossy networks
- QUIC transport layer support
- DNS covert channel transport
- v2.1 frame types for group operations

### Changed
- Improved congestion control with BBRv3
- Enhanced padding distribution algorithm
- Updated ML-KEM to latest NIST specification

### Fixed
- Race condition in multi-stream window updates
- Memory leak in transport migration
- Timing side-channel in ratchet advance

### Security
- Patched potential timing oracle in AEAD verification

## [2.0.0] - 2026-XX-XX

### Added
- Post-quantum cryptography (ML-KEM-768 hybrid)
- Polymorphic wire format for traffic analysis resistance
- Per-packet forward secrecy ratcheting
- Multi-stream multiplexing
- Connection migration between transports
- 128-bit connection IDs
- Extended frame header (24 bytes)
- Probing resistance mechanism
- Multi-transport support (UDP, TCP, WebSocket, HTTP/2)
- HKDF-BLAKE3 key derivation
- v1 compatibility mode

### Changed
- Frame header expanded from 20 to 24 bytes
- Sequence numbers expanded to 64-bit
- Payload length field expanded to 32-bit
- Key derivation labels updated for v2
- Session API now uses builder pattern
- Transport API unified to async-only
- Ratchet granularity changed from per-minute to per-packet

### Deprecated
- `Session::new()` constructor (use `Session::builder()`)
- Fixed padding classes (use continuous distribution)
- HKDF-SHA256 (use HKDF-BLAKE3)
- Sync transport API (use async)

### Removed
- Per-minute ratchet (replaced by per-packet)
- 64-bit connection ID type
- Legacy wire format

### Security
- Quantum-resistant key exchange via ML-KEM-768
- Enhanced forward secrecy with per-packet ratcheting
- Probing resistance prevents server enumeration
- Polymorphic format prevents traffic fingerprinting

## [1.6.x] - Previous Release
[See v1.x changelog]

[Unreleased]: https://github.com/wraith-protocol/wraith/compare/v2.1.0...HEAD
[2.1.0]: https://github.com/wraith-protocol/wraith/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/wraith-protocol/wraith/compare/v1.6.4...v2.0.0
```

---

## Change Categories

### Added

For new features:

```markdown
### Added
- Post-quantum cryptography with ML-KEM-768 hybrid key exchange (#123)
- Multi-stream support allowing up to 65535 concurrent streams per session
- Connection migration between transport types without session loss
- `Session::builder()` API for flexible session configuration
- `TransportManager` for managing multiple transport backends
- Group communication using TreeKEM for efficient key updates
```

### Changed

For changes to existing functionality:

```markdown
### Changed
- Frame header size increased from 20 to 24 bytes for 128-bit CID support
- Sequence numbers expanded from 32-bit to 64-bit for long-lived sessions
- Default padding strategy changed from fixed classes to continuous distribution
- `Session::send()` is now async-only; use `send_blocking()` for sync (#234)
- Key derivation migrated from HKDF-SHA256 to HKDF-BLAKE3
- Minimum supported Rust version raised to 1.85
```

### Deprecated

For features that will be removed:

```markdown
### Deprecated
- `Session::new()` constructor: use `Session::builder()` instead
- `FixedPaddingClass` enum: use `ContinuousPadding` for better obfuscation
- `hkdf_sha256()` function: use `hkdf_blake3()` for consistency
- `Transport` trait sync methods: migrate to async API
- v1 compatibility mode: will be removed in v3.0
```

### Removed

For removed features:

```markdown
### Removed
- `MinuteRatchet` struct: replaced by `PacketRatchet` for per-packet FS
- 64-bit `ConnectionId` variant: all CIDs are now 128-bit
- `AsyncTransport` trait: merged into unified `Transport` trait
- Legacy v0.x wire format support
```

### Fixed

For bug fixes:

```markdown
### Fixed
- Race condition when migrating sessions between transports (#456)
- Memory leak in stream window update handling (#457)
- Incorrect padding calculation for minimum-size packets (#458)
- Handshake timeout not properly enforced under high load (#459)
- Connection ID collision detection false positives (#460)
```

### Security

For security-related changes:

```markdown
### Security
- **CVE-2026-XXXX**: Fixed timing side-channel in AEAD tag verification
- Added quantum-resistant key exchange (ML-KEM-768) for harvest-now-decrypt-later protection
- Per-packet forward secrecy limits exposure from key compromise to single packet
- Probing resistance prevents server enumeration attacks
- Polymorphic wire format prevents traffic analysis fingerprinting
```

---

## Version Numbering

### Semantic Versioning

```
MAJOR.MINOR.PATCH

MAJOR: Breaking changes
  - Wire format incompatibility
  - API breaking changes
  - Removal of deprecated features

MINOR: New features (backward compatible)
  - New frame types
  - New transport backends
  - New APIs (additive)

PATCH: Bug fixes (backward compatible)
  - Security patches
  - Performance fixes
  - Documentation updates
```

### Version Examples

| Version | Type | Example Changes |
|---------|------|-----------------|
| 2.0.0 | Major | Hybrid PQ crypto, polymorphic format |
| 2.1.0 | Minor | Group communication, FEC |
| 2.1.1 | Patch | Security fix, bug fixes |
| 3.0.0 | Major | Remove v1 compat, new crypto |

---

## Release Notes Template

### Full Release Notes

```markdown
# WRAITH Protocol v2.1.0 Release Notes

**Release Date:** 2026-XX-XX
**Codename:** [Optional]

## Highlights

- **Group Communication**: Native support for encrypted group messaging using TreeKEM
- **Forward Error Correction**: Reliable transfer over lossy networks
- **QUIC Transport**: Modern transport with built-in encryption and multiplexing

## Breaking Changes

None in this release.

## New Features

### Group Communication (#500)

Native group messaging support using TreeKEM for efficient key management:

```rust
// Create a group
let group = Group::create(&my_identity);

// Add members
group.add_member(&alice).await?;
group.add_member(&bob).await?;

// Send to group
group.send(b"Hello everyone!").await?;
```

### Forward Error Correction (#501)

Reed-Solomon FEC for reliable transfer over lossy networks:

```rust
let session = Session::builder()
    .fec_enabled(true)
    .fec_redundancy(0.25)  // 25% redundancy
    .build()?;
```

### QUIC Transport (#502)

Full QUIC transport support with 0-RTT resumption:

```rust
let transport = QuicTransport::new(config)?;
manager.add_transport(transport);
manager.migrate(TransportType::Quic).await?;
```

## Improvements

- Congestion control upgraded to BBRv3 (#510)
- Padding distribution algorithm optimized (#511)
- Handshake latency reduced by 15% (#512)

## Bug Fixes

- Fixed race condition in multi-stream window updates (#520)
- Fixed memory leak in transport migration (#521)
- Fixed timing side-channel in ratchet advance (#522)

## Security

- Patched potential timing oracle in AEAD verification (CVE-2026-XXXX)

## Compatibility

- **Minimum Rust Version**: 1.88
- **v1 Compatibility**: Supported (deprecated, removal in v3.0)
- **v2.0 Compatibility**: Full

## Upgrade Guide

1. Update `Cargo.toml`:
   ```toml
   wraith-core = "2.1"
   ```

2. No API changes required for basic usage

3. To use new features:
   ```rust
   use wraith_core::group::Group;
   use wraith_transport::QuicTransport;
   ```

## Known Issues

- QUIC transport not yet supported on Windows ARM64 (#530)
- FEC increases memory usage by ~10% per session (#531)

## Contributors

Thanks to all contributors to this release!

- @contributor1 - Group communication implementation
- @contributor2 - FEC implementation
- @contributor3 - Bug fixes and testing

## Full Changelog

[v2.0.0...v2.1.0](https://github.com/wraith-protocol/wraith/compare/v2.0.0...v2.1.0)
```

---

## Commit Message Guidelines

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

| Type | Description |
|------|-------------|
| feat | New feature |
| fix | Bug fix |
| docs | Documentation |
| style | Formatting |
| refactor | Code restructuring |
| perf | Performance |
| test | Testing |
| chore | Maintenance |
| security | Security fix |

### Examples

```
feat(crypto): add ML-KEM-768 hybrid key exchange

Implement NIST FIPS 203 compliant ML-KEM-768 combined with X25519
for quantum-resistant key exchange. The hybrid approach ensures
security if either algorithm is compromised.

- Add MlKem768 wrapper around ml-kem crate
- Implement HybridKeyExchange trait
- Add hybrid encapsulation/decapsulation
- Update handshake to use hybrid KEM

Closes #123
```

```
fix(transport): resolve race condition in migration

Fix race condition when migrating sessions between transports
while packets are in flight. The issue occurred when the old
transport received packets after migration started.

- Add migration lock to prevent concurrent migrations
- Queue packets during migration window
- Deliver queued packets after migration complete

Fixes #456
```

```
security(crypto): patch timing oracle in AEAD verification

Fix timing side-channel in AEAD tag verification that could
leak information about tag validity. Now uses constant-time
comparison for all tag checks.

CVE-2026-XXXX
Severity: Medium
```

---

## Automation

### Changelog Generation Script

```bash
#!/bin/bash
# generate-changelog.sh

VERSION=$1
PREVIOUS_VERSION=$2

echo "## [$VERSION] - $(date +%Y-%m-%d)"
echo ""

echo "### Added"
git log $PREVIOUS_VERSION..HEAD --pretty=format:"- %s" --grep="^feat" | head -20
echo ""

echo "### Changed"
git log $PREVIOUS_VERSION..HEAD --pretty=format:"- %s" --grep="^refactor\|^perf" | head -20
echo ""

echo "### Fixed"
git log $PREVIOUS_VERSION..HEAD --pretty=format:"- %s" --grep="^fix" | head -20
echo ""

echo "### Security"
git log $PREVIOUS_VERSION..HEAD --pretty=format:"- %s" --grep="^security" | head -20
```

### GitHub Release Template

```yaml
# .github/release.yml
changelog:
  categories:
    - title: "New Features"
      labels:
        - enhancement
        - feature
    - title: "Bug Fixes"
      labels:
        - bug
        - fix
    - title: "Security"
      labels:
        - security
    - title: "Documentation"
      labels:
        - documentation
    - title: "Other Changes"
      labels:
        - "*"
```

---

## Related Documents

- [Migration Guide](09-WRAITH-Protocol-v2-Migration-Guide.md) - Version upgrade instructions
- [API Changes](16-WRAITH-Protocol-v2-API-Changes.md) - API compatibility details
- [Compatibility Matrix](15-WRAITH-Protocol-v2-Compatibility-Matrix.md) - Version compatibility

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial changelog template document |
