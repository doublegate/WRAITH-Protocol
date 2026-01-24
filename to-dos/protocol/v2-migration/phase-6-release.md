# Phase 6: Release

**Parent:** [v2 Migration Master Plan](../v2-migration-master.md)
**Version:** 1.0.0
**Status:** Planning (Post-v2.3.0)
**Story Points:** 85-100 SP
**Duration:** 1-2 weeks
**Dependencies:** Phase 5 (Client Updates)

---

## Executive Summary

Phase 6 completes the v2 migration with comprehensive documentation updates, CHANGELOG v3.0.0 preparation, security audit sign-off, and the official v3.0.0 release. This phase also establishes the compatibility mode deprecation timeline.

### Objectives

1. Complete all documentation updates
2. Prepare CHANGELOG v3.0.0
3. Obtain security audit sign-off
4. Publish performance certification
5. Tag and release v3.0.0
6. Announce compatibility mode deprecation

---

## Sprint Breakdown

### Sprint 6.1: Documentation Update (26-32 SP)

**Goal:** Synchronize all documentation with v2 implementation.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 6.1.1 | Update README.md for v2 | 3 | Critical | - |
| 6.1.2 | Update CONTRIBUTING.md | 2 | High | - |
| 6.1.3 | Update protocol specification docs | 5 | Critical | - |
| 6.1.4 | Update architecture documentation | 5 | Critical | - |
| 6.1.5 | Update API reference (rustdoc) | 5 | Critical | - |
| 6.1.6 | Update security documentation | 3 | Critical | - |
| 6.1.7 | Update client documentation | 3 | High | - |
| 6.1.8 | Create v2 migration guide | 5 | Critical | - |
| 6.1.9 | Update troubleshooting guide | 2 | Medium | - |
| 6.1.10 | Documentation review and QA | 3 | High | - |

**Acceptance Criteria:**
- [ ] All docs reflect v2 implementation
- [ ] Migration guide complete and tested
- [ ] API reference fully generated
- [ ] No outdated v1-only information
- [ ] Documentation builds without warnings

**Documentation Structure:**
```
docs/
├── architecture/
│   ├── v2-overview.md           # Updated
│   ├── v2-crypto-architecture.md # New
│   └── v2-transport-layer.md    # New
├── engineering/
│   ├── api-reference.md         # Updated
│   └── release-notes-v3.0.0.md  # New
├── security/
│   ├── security-audit-v3.0.0.md # New
│   └── threat-model-v2.md       # Updated
└── integration/
    └── migration-v1-to-v2.md    # New
```

**Code Location:** `docs/`

---

### Sprint 6.2: CHANGELOG Preparation (13-16 SP)

**Goal:** Prepare comprehensive CHANGELOG for v3.0.0.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 6.2.1 | Compile all breaking changes | 3 | Critical | - |
| 6.2.2 | Compile all new features | 3 | Critical | - |
| 6.2.3 | Compile all deprecations | 2 | Critical | - |
| 6.2.4 | Compile all security changes | 3 | Critical | - |
| 6.2.5 | Write user-friendly descriptions | 3 | High | - |
| 6.2.6 | Add migration notes per section | 2 | High | - |
| 6.2.7 | Create GitHub release notes | 2 | High | - |

**Acceptance Criteria:**
- [ ] All changes documented
- [ ] Breaking changes clearly marked
- [ ] Migration paths described
- [ ] Security improvements highlighted
- [ ] Follows Keep a Changelog format

**CHANGELOG v3.0.0 Template:**
```markdown
## [3.0.0] - 2026-XX-XX

### Added
- Post-quantum cryptography (ML-KEM-768 hybrid key exchange)
- Polymorphic wire format for traffic analysis resistance
- Per-packet forward secrecy ratcheting
- Multi-stream multiplexing (up to 65,535 streams per session)
- Connection migration between transports
- 128-bit connection IDs
- Extended frame header (24 bytes)
- Multi-transport support (UDP, TCP, WebSocket, QUIC, HTTP/2, HTTP/3)
- HKDF-BLAKE3 key derivation
- v1 compatibility mode (90-day deprecation window)
- `Session::builder()` API for flexible session configuration
- `TransportManager` for multi-transport handling
- Group communication support (TreeKEM, v2.1+)

### Changed
- Frame header expanded from 20 to 24 bytes
- Sequence numbers expanded from 32-bit to 64-bit
- Payload length field expanded to 32-bit
- All APIs now async-only (sync wrappers available)
- Minimum Rust version raised to 1.85 (Rust 2024 Edition)
- Key derivation labels updated for v2

### Deprecated
- `Session::new()` constructor (use `Session::builder()`)
- Fixed padding classes (use continuous distribution)
- `hkdf_sha256()` (use `hkdf_blake3()`)
- v1 compatibility mode (will be removed in v4.0)

### Removed
- Per-minute ratchet (replaced by per-packet)
- 64-bit ConnectionId (all CIDs now 128-bit)
- Sync-only Transport trait (merged into async)
- HKDF-SHA256 (replaced by HKDF-BLAKE3)

### Security
- Quantum-resistant key exchange via ML-KEM-768 hybrid
- Enhanced forward secrecy with per-packet ratcheting
- Probing resistance prevents server enumeration
- Polymorphic format prevents traffic fingerprinting
```

**Code Location:** `CHANGELOG.md`

---

### Sprint 6.3: Security Audit (18-23 SP)

**Goal:** Obtain security audit sign-off for v3.0.0.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 6.3.1 | Compile security checklist | 2 | Critical | - |
| 6.3.2 | Internal security review | 5 | Critical | - |
| 6.3.3 | Crypto implementation review | 5 | Critical | - |
| 6.3.4 | Vulnerability scan | 3 | Critical | - |
| 6.3.5 | Dependency audit | 3 | Critical | - |
| 6.3.6 | Security audit report | 3 | Critical | - |
| 6.3.7 | Address audit findings | 5 | Critical | - |

**Acceptance Criteria:**
- [ ] All crypto code reviewed
- [ ] No known vulnerabilities
- [ ] Dependencies audited
- [ ] Security report published
- [ ] All findings addressed

**Security Checklist:**
```
v3.0.0 Security Checklist:
══════════════════════════

Cryptography:
  [ ] Hybrid KEM implementation correct
  [ ] Key combination uses domain separation
  [ ] Per-packet ratchet provides forward secrecy
  [ ] All key material zeroized on drop
  [ ] Constant-time operations verified

Protocol:
  [ ] Handshake secure against MITM
  [ ] Replay protection working
  [ ] Version downgrade prevented
  [ ] Probing resistance effective

Implementation:
  [ ] No memory safety issues
  [ ] No timing side-channels
  [ ] Input validation complete
  [ ] Error handling secure

Dependencies:
  [ ] cargo audit clean
  [ ] No known CVEs
  [ ] Minimal dependency surface
```

**Code Location:** `docs/security/security-audit-v3.0.0.md`

---

### Sprint 6.4: Performance Certification (13-16 SP)

**Goal:** Publish official performance benchmarks for v3.0.0.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 6.4.1 | Run official benchmark suite | 3 | Critical | - |
| 6.4.2 | Document test environment | 2 | High | - |
| 6.4.3 | Compare against v1 baseline | 3 | Critical | - |
| 6.4.4 | Create performance report | 3 | Critical | - |
| 6.4.5 | Publish benchmark artifacts | 2 | High | - |
| 6.4.6 | Document performance caveats | 2 | Medium | - |

**Acceptance Criteria:**
- [ ] All performance targets met
- [ ] Comparison with v1 documented
- [ ] Test environment specified
- [ ] Results reproducible
- [ ] Caveats clearly stated

**Performance Report Template:**
```markdown
# WRAITH Protocol v3.0.0 Performance Report

## Test Environment
- CPU: AMD EPYC 7763 64-Core
- Memory: 512 GB DDR4-3200
- Network: Mellanox ConnectX-6 100GbE
- OS: Linux 6.2 (Ubuntu 24.04)
- Rust: 1.85.0

## Results

### Throughput
| Transport | v1.6 | v3.0 | Change |
|-----------|------|------|--------|
| UDP       | 300 Mbps | 520 Mbps | +73% |
| AF_XDP    | 10 Gbps | 42 Gbps | +320% |

### Latency (p99)
| Transport | v1.6 | v3.0 | Change |
|-----------|------|------|--------|
| UDP       | 2.1ms | 1.2ms | -43% |
| AF_XDP    | 450us | 380us | -16% |

### Crypto Operations
| Operation | Time | Throughput |
|-----------|------|------------|
| Hybrid KEM keygen | 0.8ms | - |
| Hybrid encap | 0.4ms | - |
| Ratchet advance | 0.3us | - |
| AEAD encrypt | - | 12 Gbps |

### Memory
| Metric | v3.0 |
|--------|------|
| Session overhead | 92 KB |
| Per-stream | 1.8 KB |
```

**Code Location:** `docs/engineering/performance-v3.0.0.md`

---

### Sprint 6.5: Release Execution (15-13 SP)

**Goal:** Tag and release v3.0.0.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 6.5.1 | Final CI/CD verification | 2 | Critical | - |
| 6.5.2 | Version bump all crates | 2 | Critical | - |
| 6.5.3 | Create release branch | 1 | Critical | - |
| 6.5.4 | Tag v3.0.0 | 1 | Critical | - |
| 6.5.5 | Publish crates to crates.io | 2 | Critical | - |
| 6.5.6 | Create GitHub release | 2 | Critical | - |
| 6.5.7 | Publish binaries | 2 | Critical | - |
| 6.5.8 | Update website/landing page | 2 | High | - |
| 6.5.9 | Announce release | 1 | High | - |

**Acceptance Criteria:**
- [ ] All CI checks pass
- [ ] Version numbers consistent
- [ ] Tag created and pushed
- [ ] Crates published
- [ ] Binaries available
- [ ] Announcement published

**Release Checklist:**
```
v3.0.0 Release Checklist:
═════════════════════════

Pre-Release:
  [ ] All tests passing
  [ ] All docs updated
  [ ] CHANGELOG complete
  [ ] Security audit signed off
  [ ] Performance benchmarks published
  [ ] Version numbers updated
  [ ] Dependencies locked

Release:
  [ ] Create release branch: release/v3.0.0
  [ ] Tag: git tag -a v3.0.0 -m "Release v3.0.0"
  [ ] Push tag: git push origin v3.0.0
  [ ] Publish to crates.io (in order):
      - wraith-core
      - wraith-crypto
      - wraith-transport
      - wraith-obfuscation
      - wraith-discovery
      - wraith-files
      - wraith-ffi
      - wraith-cli
  [ ] Create GitHub release with notes
  [ ] Upload binary artifacts

Post-Release:
  [ ] Verify crates.io publication
  [ ] Verify binary downloads
  [ ] Update website
  [ ] Send announcements
  [ ] Monitor for issues
```

**Code Location:** CI/CD workflows, release scripts

---

### Sprint 6.6: Deprecation Timeline (0 SP - Documentation Only)

**Goal:** Document and announce compatibility mode deprecation.

#### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Deprecation Notice | Official announcement of v1 compat deprecation |
| Timeline | 90-day compatibility window |
| Migration Guide | Final migration instructions |
| Support Policy | Post-deprecation support expectations |

**Deprecation Timeline:**
```
v3.0.0 Release           v4.0.0 Release
    │                         │
    ▼                         ▼
────┬─────────────────────────┬─────────────────────►
    │                         │
    │◄── 90 days compat ────►│
    │     mode active         │
    │                         │
    │  v1 compat deprecated   │ v1 compat removed
    │  (warning on use)       │ (hard error)
```

**Announcement Template:**
```markdown
# WRAITH Protocol v1 Compatibility Deprecation Notice

As of WRAITH Protocol v3.0.0, v1 compatibility mode is deprecated
and will be removed in v4.0.0 (expected Q3 2026).

## Timeline

| Date | Milestone |
|------|-----------|
| v3.0.0 Release | v1 compat deprecated (warnings enabled) |
| +30 days | First reminder notification |
| +60 days | Final reminder notification |
| +90 days | v4.0.0 release (v1 compat removed) |

## Migration Resources

- [Migration Guide](docs/integration/migration-v1-to-v2.md)
- [API Changes](docs/engineering/api-changes-v2.md)
- [Support Channel](https://github.com/doublegate/WRAITH-Protocol/discussions)

## FAQ

Q: What happens if I don't migrate?
A: After v4.0.0, v1 clients will not be able to connect.

Q: Can I get an extended compatibility window?
A: Contact support for enterprise migration assistance.
```

---

## Technical Specifications

### Version Numbers

| Crate | v2.x Version | v3.0.0 Version |
|-------|--------------|----------------|
| wraith-core | 2.2.0 | 3.0.0 |
| wraith-crypto | 2.2.0 | 3.0.0 |
| wraith-transport | 2.2.0 | 3.0.0 |
| wraith-obfuscation | 2.2.0 | 3.0.0 |
| wraith-discovery | 2.2.0 | 3.0.0 |
| wraith-files | 2.2.0 | 3.0.0 |
| wraith-ffi | 2.2.0 | 3.0.0 |
| wraith-cli | 2.2.0 | 3.0.0 |

### Release Artifacts

| Artifact | Platforms |
|----------|-----------|
| wraith-cli-linux-x86_64 | Linux amd64 |
| wraith-cli-linux-aarch64 | Linux arm64 |
| wraith-cli-macos-x86_64 | macOS Intel |
| wraith-cli-macos-aarch64 | macOS Apple Silicon |
| wraith-cli-windows-x86_64 | Windows amd64 |
| Source tarball | All |

---

## Testing Requirements

### Pre-Release Testing

| Test | Requirement |
|------|-------------|
| Full test suite | 100% pass |
| Integration tests | All platforms |
| Security tests | No vulnerabilities |
| Performance tests | Targets met |
| Documentation build | No warnings |

---

## Dependencies

### Phase Dependencies

| Dependency | Type | Notes |
|------------|------|-------|
| Phase 1-5 | Required | All implementation complete |

### External Dependencies

| Dependency | Purpose |
|------------|---------|
| crates.io | Crate publishing |
| GitHub | Release hosting |
| CI/CD | Automated release |

---

## Risk Assessment

### High Risk

| Risk | Mitigation |
|------|------------|
| Release blocker bugs | Thorough pre-release testing |
| Documentation gaps | Documentation review sprint |
| Security findings | Time buffer for fixes |

### Medium Risk

| Risk | Mitigation |
|------|------------|
| crates.io issues | Manual publication fallback |
| Binary build failures | Platform-specific CI jobs |

---

## Deliverables Checklist

### Documentation Deliverables

- [ ] README.md updated
- [ ] CHANGELOG.md v3.0.0
- [ ] Migration guide
- [ ] API reference
- [ ] Security audit report
- [ ] Performance report

### Release Deliverables

- [ ] v3.0.0 tag
- [ ] GitHub release
- [ ] crates.io publication
- [ ] Binary artifacts
- [ ] Announcement

### Process Deliverables

- [ ] Deprecation notice
- [ ] Timeline published
- [ ] Support policy documented

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial Phase 6 sprint plan |
