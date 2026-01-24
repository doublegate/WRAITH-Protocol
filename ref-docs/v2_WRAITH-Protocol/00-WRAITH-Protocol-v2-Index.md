# WRAITH Protocol v2 Documentation Index

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Reference Documentation
**Authors:** WRAITH Protocol Team

---

## Overview

This directory contains comprehensive documentation for WRAITH Protocol v2, covering the upgrade from v1, technical specifications, implementation guidance, and migration planning.

### Document Categories

| Category | Documents | Purpose |
|----------|-----------|---------|
| Specification | 01-02 | Core protocol definition |
| Planning | 03-04 | Development roadmap |
| Analysis | 05-06 | Security and advantages |
| Implementation | 07-08 | Coding guidance |
| Migration | 09-18 | Upgrade planning |

---

## Quick Reference

### For Different Audiences

| Role | Start Here | Key Documents |
|------|------------|---------------|
| **Developer** | 07-Implementation Guide | 08-API Reference, 16-API Changes |
| **Security Engineer** | 06-Security Analysis | 17-Security Considerations, 12-Crypto Upgrades |
| **Project Manager** | 04-Development Plan | 10-Implementation Phases, 03-Changelog |
| **DevOps/SRE** | 09-Migration Guide | 15-Compatibility Matrix, 14-Testing Strategy |
| **Architect** | 02-Architecture | 01-Specification, 11-Wire Format Changes |

---

## Document Catalog

### Core Specification Documents

| # | Document | Description | Pages |
|---|----------|-------------|-------|
| 01 | [WRAITH-Protocol-v2-Specification](01-WRAITH-Protocol-v2-Specification.md) | Complete technical specification for v2 protocol including wire format, cryptographic algorithms, and protocol state machines | ~40 |
| 02 | [WRAITH-Protocol-v2-Architecture](02-WRAITH-Protocol-v2-Architecture.md) | System architecture, component design, and interaction patterns | ~25 |

### Development Planning Documents

| # | Document | Description | Pages |
|---|----------|-------------|-------|
| 03 | [WRAITH-Protocol-v1-to-v2-Changelog](03-WRAITH-Protocol-v1-to-v2-Changelog.md) | Comprehensive list of changes between v1 and v2, including breaking changes | ~15 |
| 04 | [WRAITH-Protocol-v2-Development-Plan](04-WRAITH-Protocol-v2-Development-Plan.md) | 5-phase, 44-week development timeline with resource allocation | ~20 |

### Analysis Documents

| # | Document | Description | Pages |
|---|----------|-------------|-------|
| 05 | [WRAITH-Protocol-v2-Advantages](05-WRAITH-Protocol-v2-Advantages.md) | Comparative analysis of v1 vs v2 with quantified improvements | ~20 |
| 06 | [WRAITH-Protocol-v2-Security-Analysis](06-WRAITH-Protocol-v2-Security-Analysis.md) | Threat modeling, cryptographic proofs, and security properties | ~35 |

### Implementation Documents

| # | Document | Description | Pages |
|---|----------|-------------|-------|
| 07 | [WRAITH-Protocol-v2-Implementation-Guide](07-WRAITH-Protocol-v2-Implementation-Guide.md) | Practical implementation guidance with code examples | ~30 |
| 08 | [WRAITH-Protocol-v2-API-Reference](08-WRAITH-Protocol-v2-API-Reference.md) | Complete API documentation for all v2 crates | ~40 |

### Migration Documents

| # | Document | Description | Pages |
|---|----------|-------------|-------|
| 09 | [WRAITH-Protocol-v2-Migration-Guide](09-WRAITH-Protocol-v2-Migration-Guide.md) | Step-by-step v1 to v2 migration instructions | ~25 |
| 10 | [WRAITH-Protocol-v2-Implementation-Phases](10-WRAITH-Protocol-v2-Implementation-Phases.md) | Detailed sprint breakdown for v2 development | ~30 |
| 11 | [WRAITH-Protocol-v2-Wire-Format-Changes](11-WRAITH-Protocol-v2-Wire-Format-Changes.md) | Frame header changes, polymorphic encoding | ~20 |
| 12 | [WRAITH-Protocol-v2-Crypto-Upgrades](12-WRAITH-Protocol-v2-Crypto-Upgrades.md) | Hybrid PQ crypto, key derivation, ratcheting | ~25 |
| 13 | [WRAITH-Protocol-v2-Performance-Targets](13-WRAITH-Protocol-v2-Performance-Targets.md) | Throughput, latency, and resource targets | ~20 |
| 14 | [WRAITH-Protocol-v2-Testing-Strategy](14-WRAITH-Protocol-v2-Testing-Strategy.md) | Test levels, categories, CI/CD integration | ~25 |
| 15 | [WRAITH-Protocol-v2-Compatibility-Matrix](15-WRAITH-Protocol-v2-Compatibility-Matrix.md) | Version, platform, and feature compatibility | ~20 |
| 16 | [WRAITH-Protocol-v2-API-Changes](16-WRAITH-Protocol-v2-API-Changes.md) | Breaking changes, new APIs, deprecations | ~25 |
| 17 | [WRAITH-Protocol-v2-Security-Considerations](17-WRAITH-Protocol-v2-Security-Considerations.md) | Migration security, operational security | ~25 |
| 18 | [WRAITH-Protocol-v2-Changelog-Template](18-WRAITH-Protocol-v2-Changelog-Template.md) | Release documentation templates | ~15 |

---

## Recommended Reading Order

### For New Developers

1. **[01-WRAITH-Protocol-v2-Specification](01-WRAITH-Protocol-v2-Specification.md)** - Understand the protocol
2. **[02-WRAITH-Protocol-v2-Architecture](02-WRAITH-Protocol-v2-Architecture.md)** - System design
3. **[07-WRAITH-Protocol-v2-Implementation-Guide](07-WRAITH-Protocol-v2-Implementation-Guide.md)** - How to implement
4. **[08-WRAITH-Protocol-v2-API-Reference](08-WRAITH-Protocol-v2-API-Reference.md)** - API details

### For Migration Planning

1. **[03-WRAITH-Protocol-v1-to-v2-Changelog](03-WRAITH-Protocol-v1-to-v2-Changelog.md)** - What changed
2. **[05-WRAITH-Protocol-v2-Advantages](05-WRAITH-Protocol-v2-Advantages.md)** - Why upgrade
3. **[09-WRAITH-Protocol-v2-Migration-Guide](09-WRAITH-Protocol-v2-Migration-Guide.md)** - How to migrate
4. **[15-WRAITH-Protocol-v2-Compatibility-Matrix](15-WRAITH-Protocol-v2-Compatibility-Matrix.md)** - Compatibility details
5. **[16-WRAITH-Protocol-v2-API-Changes](16-WRAITH-Protocol-v2-API-Changes.md)** - Code changes needed

### For Security Review

1. **[06-WRAITH-Protocol-v2-Security-Analysis](06-WRAITH-Protocol-v2-Security-Analysis.md)** - Threat model
2. **[12-WRAITH-Protocol-v2-Crypto-Upgrades](12-WRAITH-Protocol-v2-Crypto-Upgrades.md)** - Cryptographic changes
3. **[17-WRAITH-Protocol-v2-Security-Considerations](17-WRAITH-Protocol-v2-Security-Considerations.md)** - Security practices

### For Project Planning

1. **[04-WRAITH-Protocol-v2-Development-Plan](04-WRAITH-Protocol-v2-Development-Plan.md)** - Timeline
2. **[10-WRAITH-Protocol-v2-Implementation-Phases](10-WRAITH-Protocol-v2-Implementation-Phases.md)** - Detailed phases
3. **[14-WRAITH-Protocol-v2-Testing-Strategy](14-WRAITH-Protocol-v2-Testing-Strategy.md)** - Testing approach
4. **[13-WRAITH-Protocol-v2-Performance-Targets](13-WRAITH-Protocol-v2-Performance-Targets.md)** - Success criteria

---

## Key Concepts Quick Reference

### Version Changes Summary

| Aspect | v1 | v2 |
|--------|----|----|
| Header Size | 20 bytes | 24 bytes |
| Connection ID | 64-bit | 128-bit |
| Sequence Number | 32-bit | 64-bit |
| Key Exchange | X25519 | X25519 + ML-KEM-768 |
| Wire Format | Static | Polymorphic |
| Forward Secrecy | Per-minute | Per-packet |
| Transports | UDP primary | Multi-transport |
| Platforms | Linux only | Cross-platform |

### Cryptographic Primitives

| Component | v2 Algorithm | Security Level |
|-----------|--------------|----------------|
| Key Exchange | X25519 + ML-KEM-768 | 128-bit classical + 128-bit PQ |
| AEAD | XChaCha20-Poly1305 | 256-bit |
| Hash/KDF | BLAKE3 | 256-bit |
| Signatures | Ed25519 (+ ML-DSA-65 optional) | 128-bit |
| Ratchet | BLAKE3-based chain | Per-packet FS |

### Performance Targets

| Metric | Target |
|--------|--------|
| Handshake (hybrid) | < 55ms |
| Throughput (userspace) | 500 Mbps |
| Throughput (AF_XDP) | 100 Gbps |
| Per-packet latency | < 2 microseconds |
| Memory per session | < 100 KB |

---

## Document Dependencies

```
Document Dependency Graph:
══════════════════════════

                    ┌─────────────────────┐
                    │  01-Specification   │◄────────────────────┐
                    └──────────┬──────────┘                     │
                               │                                │
              ┌────────────────┼────────────────┐               │
              ▼                ▼                ▼               │
      ┌───────────────┐ ┌───────────────┐ ┌───────────────┐     │
      │02-Architecture│ │03-Changelog   │ │04-Dev Plan    │     │
      └───────┬───────┘ └───────┬───────┘ └───────┬───────┘     │
              │                 │                 │             │
              │    ┌────────────┼────────────┐    │             │
              │    ▼            ▼            ▼    │             │
              │ ┌────────┐ ┌─────────────┐ ┌────────┐           │
              │ │05-Adv. │ │09-Migration │ │10-Phases│          │
              │ └────────┘ └──────┬──────┘ └────────┘           │
              │                   │                             │
              │    ┌──────────────┼──────────────┐              │
              ▼    ▼              ▼              ▼              │
      ┌────────────────┐  ┌────────────┐  ┌────────────┐        │
      │ 07-Impl Guide  │  │11-Wire Fmt │  │12-Crypto   │────────┘
      └───────┬────────┘  └────────────┘  └────────────┘
              │
              ▼
      ┌────────────────┐
      │ 08-API Ref     │
      └────────────────┘

      ┌────────────────┐  ┌────────────┐  ┌────────────┐
      │06-Security     │──│17-Sec.Cons.│  │14-Testing  │
      └────────────────┘  └────────────┘  └────────────┘

      ┌────────────────┐  ┌────────────┐  ┌────────────┐
      │13-Performance  │  │15-Compat   │  │16-API Chg  │
      └────────────────┘  └────────────┘  └────────────┘

                          ┌────────────┐
                          │18-Changelog│
                          │  Template  │
                          └────────────┘
```

---

## Related Resources

### v1 Documentation

Located in `docs/architecture/`:
- `protocol-overview.md` - v1 protocol design
- `layer-design.md` - v1 layer specifications
- `security-model.md` - v1 threat model
- `performance-architecture.md` - v1 performance targets
- `network-topology.md` - v1 network design

### Implementation

- `crates/` - Rust workspace with all protocol crates
- `clients/` - Client applications (Transfer, Chat, Android, iOS, Sync)
- `tests/` - Integration tests and benchmarks

### Operations

- `docs/operations/` - Deployment and operations guides
- `docs/troubleshooting/` - Known issues and fixes
- `docs/security/` - Security audit reports

---

## Contributing

### Adding Documents

1. Follow the naming convention: `##-DOCUMENT-NAME.md`
2. Use the standard template structure
3. Include version, date, status, and authors
4. Add to this 00-WRAITH-Protocol-v2-Index.md
5. Update dependency graph if applicable

### Updating Documents

1. Update the version number
2. Update the date
3. Add changelog entry at bottom
4. Cross-reference related document updates

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial documentation set with 18 documents |

---

## Quick Links

| Need To... | Document |
|------------|----------|
| Understand v2 protocol | [01-Specification](01-WRAITH-Protocol-v2-Specification.md) |
| Plan migration | [09-Migration Guide](09-WRAITH-Protocol-v2-Migration-Guide.md) |
| Review security | [06-Security Analysis](06-WRAITH-Protocol-v2-Security-Analysis.md) |
| Update code | [16-API Changes](16-WRAITH-Protocol-v2-API-Changes.md) |
| Check compatibility | [15-Compatibility Matrix](15-WRAITH-Protocol-v2-Compatibility-Matrix.md) |
| Implement features | [07-Implementation Guide](07-WRAITH-Protocol-v2-Implementation-Guide.md) |
| Run tests | [14-Testing Strategy](14-WRAITH-Protocol-v2-Testing-Strategy.md) |
| Track progress | [04-Development Plan](04-WRAITH-Protocol-v2-Development-Plan.md) |
