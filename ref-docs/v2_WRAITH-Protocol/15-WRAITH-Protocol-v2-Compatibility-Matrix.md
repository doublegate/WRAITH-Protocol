# WRAITH Protocol v2 Compatibility Matrix

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Planning Document
**Authors:** WRAITH Protocol Team

---

## Table of Contents

1. [Overview](#overview)
2. [Version Compatibility](#version-compatibility)
3. [Platform Support](#platform-support)
4. [Transport Compatibility](#transport-compatibility)
5. [Cryptographic Compatibility](#cryptographic-compatibility)
6. [Feature Compatibility](#feature-compatibility)
7. [Client Compatibility](#client-compatibility)
8. [Dependency Compatibility](#dependency-compatibility)
9. [Migration Paths](#migration-paths)

---

## Overview

This document defines compatibility relationships between WRAITH Protocol versions, platforms, transports, and features. It serves as a reference for deployment planning and migration decisions.

### Compatibility Principles

1. **Backward Compatibility:** v2 servers SHOULD support v1 clients in compatibility mode
2. **Forward Compatibility:** v1 clients connecting to v2 servers get v1 semantics
3. **Graceful Degradation:** Missing features result in fallback, not failure
4. **Clear Boundaries:** Incompatibilities are documented and detectable

---

## Version Compatibility

### Protocol Version Matrix

```
                          Server Version
                    ┌─────────┬─────────┬─────────┐
                    │  v1.x   │  v2.0   │  v2.1   │
           ┌────────┼─────────┼─────────┼─────────┤
           │ v1.0   │   ✓     │   C     │   C     │
Client     │ v1.5   │   ✓     │   C     │   C     │
Version    │ v1.6   │   ✓     │   C     │   C     │
           │ v2.0   │   C     │   ✓     │   ✓     │
           │ v2.1   │   C     │   ✓     │   ✓     │
           └────────┴─────────┴─────────┴─────────┘

Legend:
  ✓ = Full compatibility
  C = Compatibility mode (reduced features)
  ✗ = Incompatible
```

### Version Negotiation

```rust
/// Version negotiation result
pub enum NegotiatedVersion {
    /// Native v1 protocol
    V1Native,

    /// Native v2 protocol with full features
    V2Native,

    /// v2 server in v1 compatibility mode
    V2CompatV1,

    /// v2 with classical crypto only (no PQ)
    V2ClassicalOnly,
}

/// Negotiation logic
pub fn negotiate(
    client_versions: &[Version],
    server_versions: &[Version],
    client_features: FeatureSet,
    server_features: FeatureSet,
) -> Result<NegotiatedVersion, NegotiationError> {
    // Find highest common version
    let common = client_versions.iter()
        .filter(|v| server_versions.contains(v))
        .max();

    match common {
        Some(Version::V2_1) | Some(Version::V2_0) => {
            // Check for PQ support
            if client_features.contains(Feature::PostQuantum)
                && server_features.contains(Feature::PostQuantum)
            {
                Ok(NegotiatedVersion::V2Native)
            } else {
                Ok(NegotiatedVersion::V2ClassicalOnly)
            }
        }
        Some(Version::V1_X) => Ok(NegotiatedVersion::V1Native),
        None => {
            // Try compatibility mode
            if server_versions.iter().any(|v| v.major() == 2)
                && client_versions.iter().any(|v| v.major() == 1)
            {
                Ok(NegotiatedVersion::V2CompatV1)
            } else {
                Err(NegotiationError::NoCommonVersion)
            }
        }
    }
}
```

### Feature Availability by Version

| Feature | v1.0 | v1.5 | v1.6 | v2.0 | v2.1 |
|---------|------|------|------|------|------|
| Basic Transfer | Yes | Yes | Yes | Yes | Yes |
| Obfuscation | Yes | Yes | Yes | Yes | Yes |
| AF_XDP | No | Partial | Yes | Yes | Yes |
| Multi-stream | No | No | No | Yes | Yes |
| Post-Quantum | No | No | No | Yes | Yes |
| Polymorphic Format | No | No | No | Yes | Yes |
| Per-packet FS | No | No | No | Yes | Yes |
| Connection Migration | No | No | Partial | Yes | Yes |
| Group Communication | No | No | No | No | Yes |
| FEC | No | No | No | No | Yes |

---

## Platform Support

### Operating System Compatibility

| Platform | v1.x | v2.0 | v2.1 | Notes |
|----------|------|------|------|-------|
| Linux x86_64 | Full | Full | Full | Primary platform |
| Linux aarch64 | Full | Full | Full | ARM64 support |
| Linux musl | Partial | Full | Full | Static linking |
| macOS x86_64 | Partial | Full | Full | No AF_XDP |
| macOS aarch64 | Partial | Full | Full | Apple Silicon |
| Windows x86_64 | Limited | Full | Full | No AF_XDP |
| FreeBSD | Limited | Partial | Full | Community support |
| Android | None | Partial | Full | Via NDK |
| iOS | None | Partial | Full | Via UniFFI |
| WASM | None | Partial | Partial | Browser support |

### Kernel Requirements

| Feature | Linux | macOS | Windows |
|---------|-------|-------|---------|
| Basic UDP | 4.0+ | 10.15+ | 10+ |
| io_uring | 5.1+ | N/A | N/A |
| AF_XDP | 5.3+ | N/A | N/A |
| eBPF | 4.4+ | N/A | N/A |
| QUIC | 5.6+ | 11+ | 11+ |

### Architecture Support

```
Architecture Support Matrix:
════════════════════════════

              ┌──────────────────────────────────────────────┐
              │               Feature Support                 │
              ├─────────┬─────────┬─────────┬────────────────┤
Architecture  │ Basic   │ io_uring│ AF_XDP  │ SIMD Crypto    │
──────────────┼─────────┼─────────┼─────────┼────────────────┤
x86_64        │   ✓     │   ✓     │   ✓     │ AVX2, AVX-512  │
aarch64       │   ✓     │   ✓     │   ✓     │ NEON, SVE      │
riscv64       │   ✓     │   ✓     │   ✓     │ Vector ext     │
wasm32        │   ✓     │   ✗     │   ✗     │ SIMD128        │
armv7         │   ✓     │   ✗     │   ✗     │ NEON           │
──────────────┴─────────┴─────────┴─────────┴────────────────┘
```

---

## Transport Compatibility

### Transport Matrix

| Transport | v1.x | v2.0 | v2.1 | Platforms |
|-----------|------|------|------|-----------|
| UDP | Primary | Yes | Yes | All |
| TCP | Fallback | Yes | Yes | All |
| WebSocket | No | Yes | Yes | All |
| QUIC | No | Yes | Yes | Linux 5.6+, macOS 11+ |
| HTTP/2 | No | Yes | Yes | All |
| HTTP/3 | No | No | Yes | Linux 5.6+, macOS 11+ |
| AF_XDP | Partial | Yes | Yes | Linux 5.3+ |
| Covert (DNS) | No | Partial | Yes | All |
| Covert (ICMP) | No | Partial | Yes | Root required |

### Transport Negotiation

```rust
/// Transport capability advertisement
#[derive(Clone, Debug)]
pub struct TransportCapabilities {
    /// Supported transports in preference order
    pub transports: Vec<TransportType>,

    /// UDP configuration
    pub udp: Option<UdpCapabilities>,

    /// QUIC configuration
    pub quic: Option<QuicCapabilities>,

    /// WebSocket endpoint (if available)
    pub websocket_endpoint: Option<String>,
}

/// Select best common transport
pub fn select_transport(
    client: &TransportCapabilities,
    server: &TransportCapabilities,
) -> Option<TransportType> {
    // Prefer client's ordering
    for transport in &client.transports {
        if server.transports.contains(transport) {
            return Some(*transport);
        }
    }
    None
}
```

### Transport Feature Matrix

| Feature | UDP | TCP | WebSocket | QUIC | HTTP/2 |
|---------|-----|-----|-----------|------|--------|
| Zero-RTT | No | No | No | Yes | No |
| Multiplexing | App | No | No | Native | Native |
| Migration | Yes | No | No | Yes | No |
| Kernel Bypass | Yes | No | No | Limited | No |
| Firewall Friendly | Maybe | Yes | Yes | Maybe | Yes |
| Web Browser | No | No | Yes | No | Yes |

---

## Cryptographic Compatibility

### Algorithm Support

| Algorithm | v1.x | v2.0 | v2.1 | Notes |
|-----------|------|------|------|-------|
| X25519 | Yes | Yes | Yes | Key exchange |
| ML-KEM-768 | No | Yes | Yes | Post-quantum KEM |
| ML-KEM-1024 | No | No | Yes | Higher security |
| Ed25519 | Yes | Yes | Yes | Signatures |
| ML-DSA-65 | No | Optional | Optional | PQ signatures |
| XChaCha20-Poly1305 | Yes | Yes | Yes | AEAD |
| AES-256-GCM | Partial | Yes | Yes | Hardware support |
| BLAKE3 | Yes | Yes | Yes | Hashing, KDF |
| SHA-256 | Partial | Deprecated | Deprecated | Legacy |

### Cryptographic Modes

```
Cryptographic Mode Compatibility:
═════════════════════════════════

Mode                 v1 Client   v2 Classical   v2 Hybrid
─────────────────────────────────────────────────────────
v1 Server            ✓           ✓ (compat)     ✗
v2 Server (compat)   ✓           ✓              ✓
v2 Server (strict)   ✗           ✓              ✓

Security Levels:
─────────────────────────────────────────────────────────
Mode              Classical    Post-Quantum   Forward Secrecy
v1 Native         128-bit      None           Per-minute
v2 Classical      128-bit      None           Per-packet
v2 Hybrid         128-bit      128-bit        Per-packet
```

### Key Compatibility

```rust
/// Key format compatibility
pub struct KeyCompatibility;

impl KeyCompatibility {
    /// Check if v1 key can be used in v2
    pub fn v1_key_in_v2(key: &V1Key) -> CompatibilityResult {
        match key {
            V1Key::X25519(k) => {
                // X25519 keys are fully compatible
                CompatibilityResult::Compatible
            }
            V1Key::Ed25519(k) => {
                // Ed25519 can be used but should add PQ binding
                CompatibilityResult::CompatibleWithWarning(
                    "Consider adding ML-DSA binding for PQ security"
                )
            }
        }
    }

    /// Migrate v1 identity to v2
    pub fn migrate_identity(v1_identity: &V1Identity) -> V2Identity {
        V2Identity {
            classical: v1_identity.clone(),
            post_quantum: None,  // Can be added later
            binding_proof: None,
        }
    }
}
```

---

## Feature Compatibility

### Feature Dependency Graph

```
Feature Dependencies:
═════════════════════

                    ┌─────────────────┐
                    │  Group Comm     │
                    │   (TreeKEM)     │
                    └────────┬────────┘
                             │ requires
                    ┌────────▼────────┐
                    │ Multi-Stream    │
                    └────────┬────────┘
                             │ requires
        ┌────────────────────┼────────────────────┐
        │                    │                    │
┌───────▼───────┐  ┌─────────▼─────────┐  ┌───────▼───────┐
│  Per-Packet   │  │   Polymorphic    │  │  Connection   │
│   FS Ratchet  │  │   Wire Format    │  │   Migration   │
└───────┬───────┘  └─────────┬─────────┘  └───────┬───────┘
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │ requires
                    ┌────────▼────────┐
                    │   v2 Protocol   │
                    │   Core          │
                    └────────┬────────┘
                             │ fallback
                    ┌────────▼────────┐
                    │   v1 Protocol   │
                    │   (Compat)      │
                    └─────────────────┘
```

### Feature Negotiation

```rust
/// Feature flags for negotiation
bitflags! {
    pub struct Features: u64 {
        // Core features
        const BASIC_TRANSFER     = 1 << 0;
        const OBFUSCATION        = 1 << 1;

        // v2 features
        const MULTI_STREAM       = 1 << 10;
        const PER_PACKET_FS      = 1 << 11;
        const POLYMORPHIC_FORMAT = 1 << 12;
        const CONNECTION_MIGRATE = 1 << 13;
        const POST_QUANTUM       = 1 << 14;

        // v2.1 features
        const GROUP_COMMUNICATION = 1 << 20;
        const FEC                 = 1 << 21;
        const QUIC_TRANSPORT      = 1 << 22;
        const COVERT_DNS          = 1 << 23;

        // Transport features
        const AF_XDP             = 1 << 30;
        const IO_URING           = 1 << 31;

        // Feature sets
        const V1_FEATURES = Self::BASIC_TRANSFER.bits() | Self::OBFUSCATION.bits();
        const V2_FEATURES = Self::V1_FEATURES.bits()
            | Self::MULTI_STREAM.bits()
            | Self::PER_PACKET_FS.bits()
            | Self::POLYMORPHIC_FORMAT.bits()
            | Self::POST_QUANTUM.bits();
    }
}

/// Determine effective features
pub fn negotiate_features(
    client: Features,
    server: Features,
    version: NegotiatedVersion,
) -> Features {
    let common = client & server;

    match version {
        NegotiatedVersion::V1Native | NegotiatedVersion::V2CompatV1 => {
            common & Features::V1_FEATURES
        }
        NegotiatedVersion::V2ClassicalOnly => {
            common & Features::V2_FEATURES & !Features::POST_QUANTUM
        }
        NegotiatedVersion::V2Native => {
            common & Features::V2_FEATURES
        }
    }
}
```

---

## Client Compatibility

### Client Application Matrix

| Client | v1.x Server | v2.0 Server | v2.1 Server | Notes |
|--------|-------------|-------------|-------------|-------|
| wraith-transfer v1 | Full | Compat | Compat | Desktop file transfer |
| wraith-transfer v2 | Compat | Full | Full | Desktop file transfer |
| wraith-chat v2 | No | Full | Full | Requires v2 for E2EE |
| wraith-android v2 | No | Full | Full | Mobile client |
| wraith-ios v2 | No | Full | Full | Mobile client |
| wraith-sync v2 | No | Full | Full | File synchronization |
| wraith-cli v1 | Full | Compat | Compat | Command-line interface |
| wraith-cli v2 | Compat | Full | Full | Command-line interface |

### API Compatibility

```rust
/// API version compatibility check
pub fn check_api_compatibility(
    client_api: ApiVersion,
    server_api: ApiVersion,
) -> ApiCompatibility {
    match (client_api.major, server_api.major) {
        // Same major version - fully compatible
        (a, b) if a == b => ApiCompatibility::Full,

        // Client v2, Server v1 - limited (if server has compat)
        (2, 1) => ApiCompatibility::Limited {
            unavailable: vec![
                "multi_stream",
                "group_communication",
                "post_quantum",
            ],
        },

        // Client v1, Server v2 - works with compat mode
        (1, 2) => ApiCompatibility::Degraded {
            warning: "v1 client missing v2 security features",
        },

        // Incompatible versions
        _ => ApiCompatibility::Incompatible,
    }
}
```

---

## Dependency Compatibility

### Rust Version Requirements

| WRAITH Version | MSRV | Rust Edition | Notes |
|----------------|------|--------------|-------|
| v1.0 - v1.5 | 1.70 | 2021 | Stable |
| v1.6 | 1.75 | 2021 | async_fn_in_trait |
| v2.0 | 1.85 | 2024 | New edition |
| v2.1 | 1.85 | 2024 | Same as v2.0 |
| v2.2 | 1.88 | 2024 | MSRV upgrade |

### Key Dependencies

| Dependency | v1.x | v2.0 | v2.1 | Purpose |
|------------|------|------|------|---------|
| tokio | 1.28 | 1.35 | 1.40 | Async runtime |
| snow | 0.9 | 0.9 | 0.9 | Noise protocol |
| x25519-dalek | 1.2 | 2.0 | 2.0 | X25519 |
| ml-kem | - | 0.1 | 0.2 | Post-quantum KEM |
| blake3 | 1.3 | 1.5 | 1.5 | Hashing |
| chacha20poly1305 | 0.10 | 0.10 | 0.10 | AEAD |
| io-uring | 0.5 | 0.6 | 0.6 | Linux io_uring |

### Breaking Dependency Changes

```rust
/// Known breaking changes in dependencies
pub mod breaking_changes {
    /// x25519-dalek 1.x to 2.x
    ///
    /// Change: `StaticSecret` API changed
    /// Migration:
    /// - Old: `let pk = PublicKey::from(&sk)`
    /// - New: `let pk = PublicKey::from(&sk)` (same, but types differ)
    ///
    /// Impact: Recompile required, no API changes in WRAITH

    /// tokio 1.28 to 1.35
    ///
    /// Change: New `select!` semantics
    /// Migration: Review select! usage
    ///
    /// Impact: Minor code changes in transport layer
}
```

---

## Migration Paths

### Supported Migration Paths

```
Migration Path Diagram:
═══════════════════════

v1.0 ──────────────────────────────────────────────────┐
  │                                                     │
  ▼                                                     │
v1.5 ────────────────────────────────────┐              │
  │                                       │              │
  ▼                                       ▼              ▼
v1.6 ──────────────► v2.0 ──────────────► v2.1 ◄────────┘
         │            │
         │            │
         │ (compat)   │ (compat)
         ▼            ▼
     [v1 clients] [v1 clients]

Supported:
  v1.x → v2.0 (with compat mode)
  v1.x → v2.1 (with compat mode)
  v2.0 → v2.1 (direct upgrade)
  v1.x → v1.x+1 (minor upgrade)

Not Supported:
  v2.x → v1.x (downgrade)
```

### Migration Configuration

```rust
/// Migration configuration options
pub struct MigrationConfig {
    /// Source version
    pub from_version: Version,

    /// Target version
    pub to_version: Version,

    /// Compatibility mode settings
    pub compat: CompatibilitySettings,

    /// Key migration options
    pub key_migration: KeyMigrationOptions,

    /// Session migration options
    pub session_migration: SessionMigrationOptions,
}

impl MigrationConfig {
    /// Standard v1 to v2 migration
    pub fn v1_to_v2() -> Self {
        Self {
            from_version: Version::V1_6,
            to_version: Version::V2_1,
            compat: CompatibilitySettings {
                enable_v1_compat: true,
                v1_timeout: Duration::from_days(90),
            },
            key_migration: KeyMigrationOptions {
                keep_v1_identity: true,
                generate_pq_keys: true,
                create_binding_proof: true,
            },
            session_migration: SessionMigrationOptions {
                migrate_active_sessions: true,
                preserve_session_data: true,
            },
        }
    }
}
```

### Compatibility Mode Duration

| Migration | Recommended Compat Period | Maximum |
|-----------|---------------------------|---------|
| v1.x → v2.0 | 90 days | 180 days |
| v2.0 → v2.1 | 30 days | 60 days |
| Minor versions | 14 days | 30 days |

---

## Related Documents

- [Migration Guide](09-WRAITH-Protocol-v2-Migration-Guide.md) - Step-by-step migration instructions
- [Changelog](03-WRAITH-Protocol-v1-to-v2-Changelog.md) - Detailed change history
- [API Changes](16-WRAITH-Protocol-v2-API-Changes.md) - API compatibility details
- [Crypto Upgrades](12-WRAITH-Protocol-v2-Crypto-Upgrades.md) - Cryptographic changes

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial compatibility matrix document |
