# WRAITH Protocol v1 to v2 Migration Guide

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Planning Document
**Authors:** WRAITH Protocol Team

---

## Table of Contents

1. [Overview](#overview)
2. [Migration Strategy](#migration-strategy)
3. [Pre-Migration Checklist](#pre-migration-checklist)
4. [Breaking Changes Summary](#breaking-changes-summary)
5. [Step-by-Step Migration](#step-by-step-migration)
6. [Crate-by-Crate Migration](#crate-by-crate-migration)
7. [Wire Protocol Migration](#wire-protocol-migration)
8. [Cryptographic Migration](#cryptographic-migration)
9. [API Migration](#api-migration)
10. [Testing Your Migration](#testing-your-migration)
11. [Rollback Procedures](#rollback-procedures)
12. [Troubleshooting](#troubleshooting)

---

## Overview

This guide provides step-by-step instructions for migrating WRAITH Protocol implementations from v1 to v2. The migration involves significant architectural changes including post-quantum cryptography, polymorphic wire formats, and cross-platform support.

### Migration Complexity Assessment

| Component | Complexity | Effort | Risk |
|-----------|------------|--------|------|
| Wire Format | High | 3-4 weeks | High |
| Cryptography | High | 4-6 weeks | Critical |
| Transport Layer | Medium | 2-3 weeks | Medium |
| Session Management | Medium | 2-3 weeks | Medium |
| Obfuscation | Medium | 2-3 weeks | Low |
| File Transfer | Low | 1-2 weeks | Low |
| CLI/API | Medium | 2-3 weeks | Low |

### Compatibility Matrix

```
┌─────────────────────────────────────────────────────────────┐
│                    Compatibility Matrix                      │
├─────────────┬─────────────┬─────────────┬──────────────────┤
│             │  v1 Client  │  v2 Client  │ v2 (Compat Mode) │
├─────────────┼─────────────┼─────────────┼──────────────────┤
│ v1 Server   │     ✓       │     ✗       │       ✓          │
│ v2 Server   │     ✗       │     ✓       │       ✓          │
│ v2 (Compat) │     ✓       │     ✓       │       ✓          │
└─────────────┴─────────────┴─────────────┴──────────────────┘
```

---

## Migration Strategy

### Recommended Approach: Parallel Deployment

We recommend a phased migration using parallel deployment:

```
Phase 1: Deploy v2 with compatibility mode
         ┌─────────────┐
         │   v2 Node   │◄──── Accepts v1 and v2 connections
         │ (Compat ON) │
         └─────────────┘
              │
    ┌─────────┴─────────┐
    ▼                   ▼
┌─────────┐       ┌─────────┐
│v1 Client│       │v2 Client│
└─────────┘       └─────────┘

Phase 2: Migrate clients to v2
         ┌─────────────┐
         │   v2 Node   │
         │ (Compat ON) │
         └─────────────┘
              │
    ┌─────────┴─────────┐
    ▼                   ▼
┌─────────┐       ┌─────────┐
│v2 Client│       │v2 Client│
└─────────┘       └─────────┘

Phase 3: Disable compatibility mode
         ┌─────────────┐
         │   v2 Node   │◄──── v2 only (enhanced security)
         │(Compat OFF) │
         └─────────────┘
              │
    ┌─────────┴─────────┐
    ▼                   ▼
┌─────────┐       ┌─────────┐
│v2 Client│       │v2 Client│
└─────────┘       └─────────┘
```

### Alternative Strategies

1. **Big Bang Migration**: Upgrade all nodes simultaneously
   - Pros: Clean cut, no compatibility overhead
   - Cons: High risk, requires coordination

2. **Gradual Rollout**: Upgrade nodes one at a time
   - Pros: Low risk, easy rollback
   - Cons: Extended compatibility period

3. **Feature Flags**: Enable v2 features incrementally
   - Pros: Fine-grained control
   - Cons: Code complexity

---

## Pre-Migration Checklist

### System Requirements

- [ ] Rust 1.88+ installed (MSRV for v2)
- [ ] Linux 6.2+ kernel (for AF_XDP support)
- [ ] 16GB+ RAM recommended for build
- [ ] Network access for crate downloads

### Dependency Audit

```bash
# Check current dependencies
cargo tree -p wraith-core

# Verify no conflicting versions
cargo update --dry-run

# Run security audit
cargo audit
```

### Backup Procedures

- [ ] Backup all configuration files
- [ ] Export existing key material (encrypted)
- [ ] Document current session states
- [ ] Snapshot persistent storage

### Version Verification

```rust
// Verify current version
use wraith_core::VERSION;
assert!(VERSION.starts_with("1."));

// Check feature flags
#[cfg(feature = "v1-compat")]
compile_error!("v1-compat feature should not be enabled on v1");
```

---

## Breaking Changes Summary

### Wire Format Changes

| Aspect | v1 | v2 | Migration Impact |
|--------|----|----|------------------|
| Header Size | 20 bytes | 24 bytes | Frame parser update |
| Version Field | 4 bits | 8 bits | Version negotiation |
| Connection ID | 64-bit | 128-bit | Session management |
| Padding | Fixed classes | Continuous | Obfuscation engine |
| Format | Static | Polymorphic | Complete rewrite |

### Cryptographic Changes

| Component | v1 | v2 | Migration Impact |
|-----------|----|----|------------------|
| Key Exchange | X25519 | X25519 + ML-KEM-768 | Hybrid negotiation |
| Key Derivation | HKDF-SHA256 | HKDF-BLAKE3 | Label changes |
| Ratchet | Per-minute/1M | Per-packet | Performance tuning |
| Signatures | Ed25519 | Ed25519 + ML-DSA-65 | Identity migration |

### API Changes

| Area | Change Type | Description |
|------|-------------|-------------|
| `Session::new()` | Breaking | New parameters required |
| `Frame::encode()` | Breaking | Polymorphic encoding |
| `Handshake` | Breaking | New hybrid protocol |
| `Config` | Additive | New optional fields |

---

## Step-by-Step Migration

### Step 1: Update Cargo.toml

```toml
# Before (v1)
[dependencies]
wraith-core = "1.6"
wraith-crypto = "1.6"
wraith-transport = "1.6"

# After (v2)
[dependencies]
wraith-core = { version = "2.0", features = ["v1-compat"] }
wraith-crypto = { version = "2.0", features = ["hybrid-pq"] }
wraith-transport = { version = "2.0", features = ["multi-transport"] }
```

### Step 2: Update Feature Flags

```toml
[features]
default = ["v2-native"]

# Enable during migration
v1-compat = [
    "wraith-core/v1-compat",
    "wraith-crypto/classical-only",
    "wraith-transport/udp-only",
]

# Final configuration
v2-native = [
    "wraith-core/polymorphic",
    "wraith-crypto/hybrid-pq",
    "wraith-transport/multi-transport",
]
```

### Step 3: Migrate Configuration

```rust
// v1 Configuration
let config_v1 = Config {
    listen_addr: "0.0.0.0:8443".parse()?,
    key_path: PathBuf::from("/etc/wraith/keys"),
    max_sessions: 1000,
};

// v2 Configuration (with compatibility)
let config_v2 = ConfigBuilder::new()
    .listen_addr("0.0.0.0:8443".parse()?)
    .key_path(PathBuf::from("/etc/wraith/keys"))
    .max_sessions(1000)
    // New v2 options
    .enable_v1_compat(true)  // Enable during migration
    .crypto_mode(CryptoMode::HybridPQ)
    .transport_mode(TransportMode::MultiProtocol)
    .wire_format(WireFormat::Polymorphic)
    .build()?;
```

### Step 4: Update Session Management

```rust
// v1 Session Creation
let session = Session::new(
    connection_id,
    peer_addr,
    shared_secret,
)?;

// v2 Session Creation
let session = SessionBuilder::new()
    .connection_id(ConnectionId::new_v2())  // 128-bit
    .peer_addr(peer_addr)
    .crypto_context(CryptoContext::hybrid(
        classical_secret,
        pq_secret,
    ))
    .wire_format(session_derived_format)
    .transport(TransportHandle::detect(peer_addr)?)
    .build()?;
```

### Step 5: Update Handshake Protocol

```rust
// v1 Handshake
let handshake = NoiseXX::new(keypair);
let (session_keys, _) = handshake.complete(socket)?;

// v2 Handshake
let handshake = HybridHandshake::new(
    ClassicalKeypair::from(keypair),
    PQKeypair::generate()?,
);

// Negotiate version with peer
let negotiated = handshake.negotiate_version(socket)?;

let session_keys = match negotiated {
    Version::V1 => handshake.complete_v1_compat(socket)?,
    Version::V2 => handshake.complete_hybrid(socket)?,
};
```

### Step 6: Update Frame Processing

```rust
// v1 Frame Encoding
let frame = Frame::new(FrameType::Data, payload);
let encoded = frame.encode(&session_keys)?;

// v2 Frame Encoding (Polymorphic)
let frame = FrameBuilder::new()
    .frame_type(FrameType::Data)
    .payload(payload)
    .build()?;

// Encoding uses session-derived format
let encoded = session.wire_format().encode(&frame, &session.crypto())?;
```

---

## Crate-by-Crate Migration

### wraith-core Migration

**Key Changes:**
- Frame header expanded from 20 to 24 bytes
- Connection IDs now 128-bit
- Session state machine updated
- BBRv3 congestion control

```rust
// Migration helper for connection IDs
impl From<v1::ConnectionId> for v2::ConnectionId {
    fn from(v1_cid: v1::ConnectionId) -> Self {
        // Expand 64-bit to 128-bit with migration prefix
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&v1_cid.to_bytes());
        bytes[8..12].copy_from_slice(b"MIG1");
        Self::from_bytes(bytes)
    }
}
```

### wraith-crypto Migration

**Key Changes:**
- Hybrid key exchange (X25519 + ML-KEM-768)
- Per-packet forward secrecy
- New key derivation labels
- ML-DSA-65 signatures (optional)

```rust
// Hybrid key exchange
pub struct HybridKeyExchange {
    classical: X25519KeyExchange,
    post_quantum: MlKem768KeyExchange,
}

impl HybridKeyExchange {
    pub fn encapsulate(&self, peer_public: &HybridPublicKey)
        -> Result<(SharedSecret, HybridCiphertext)>
    {
        let (classical_ss, classical_ct) =
            self.classical.encapsulate(&peer_public.classical)?;
        let (pq_ss, pq_ct) =
            self.post_quantum.encapsulate(&peer_public.post_quantum)?;

        // Combine shared secrets
        let combined = blake3::keyed_hash(
            b"wraith-hybrid-kem-v2-combine-ss",
            &[classical_ss.as_bytes(), pq_ss.as_bytes()].concat(),
        );

        Ok((
            SharedSecret::from(combined.as_bytes()),
            HybridCiphertext { classical_ct, pq_ct },
        ))
    }
}
```

### wraith-transport Migration

**Key Changes:**
- Multi-transport support (UDP, TCP, WebSocket, QUIC)
- Transport abstraction layer
- Connection migration between transports

```rust
// v2 Transport Abstraction
pub trait Transport: Send + Sync {
    async fn send(&self, data: &[u8]) -> Result<()>;
    async fn recv(&self, buf: &mut [u8]) -> Result<usize>;
    fn transport_type(&self) -> TransportType;
    fn supports_migration(&self) -> bool;
}

// Multi-transport manager
pub struct TransportManager {
    transports: HashMap<TransportType, Box<dyn Transport>>,
    primary: TransportType,
    fallback_order: Vec<TransportType>,
}
```

### wraith-obfuscation Migration

**Key Changes:**
- Continuous padding distributions (vs fixed classes)
- Adaptive timing with Markov models
- Enhanced protocol mimicry

```rust
// v2 Continuous padding
pub struct ContinuousPadding {
    distribution: PaddingDistribution,
    min_size: usize,
    max_size: usize,
}

impl ContinuousPadding {
    pub fn sample(&self, payload_size: usize) -> usize {
        let target = self.distribution.sample();
        target.saturating_sub(payload_size)
            .clamp(0, self.max_size - payload_size)
    }
}
```

---

## Wire Protocol Migration

### Frame Header Migration

```
v1 Frame Header (20 bytes):
┌────────────┬────────────┬────────────┬────────────┐
│  Version   │   Type     │   Flags    │  Reserved  │
│  (4 bits)  │  (4 bits)  │  (8 bits)  │  (16 bits) │
├────────────┴────────────┴────────────┴────────────┤
│                   Sequence (32 bits)               │
├───────────────────────────────────────────────────┤
│                    Length (16 bits)                │
├───────────────────────────────────────────────────┤
│                Connection ID (64 bits)             │
└───────────────────────────────────────────────────┘

v2 Frame Header (24 bytes):
┌────────────┬────────────┬────────────┬────────────┐
│  Version   │   Type     │   Flags    │  Reserved  │
│  (8 bits)  │  (8 bits)  │  (8 bits)  │  (8 bits)  │
├────────────┴────────────┴────────────┴────────────┤
│                   Sequence (64 bits)               │
├───────────────────────────────────────────────────┤
│                    Length (32 bits)                │
├───────────────────────────────────────────────────┤
│               Connection ID (128 bits)             │
│                    (first 64 bits)                 │
├───────────────────────────────────────────────────┤
│               Connection ID (128 bits)             │
│                    (last 64 bits)                  │
└───────────────────────────────────────────────────┘
```

### Version Negotiation

```rust
// Version negotiation during handshake
pub async fn negotiate_version(
    socket: &mut impl AsyncReadWrite,
    supported: &[Version],
) -> Result<Version> {
    // Send supported versions
    let offer = VersionOffer::new(supported);
    socket.write_all(&offer.encode()).await?;

    // Receive peer's selection
    let mut buf = [0u8; 4];
    socket.read_exact(&mut buf).await?;
    let selected = Version::decode(&buf)?;

    if !supported.contains(&selected) {
        return Err(Error::VersionMismatch);
    }

    Ok(selected)
}
```

---

## Cryptographic Migration

### Key Material Migration

```rust
// Migrate v1 identity to v2
pub fn migrate_identity(
    v1_keypair: &ed25519::Keypair,
) -> Result<HybridIdentity> {
    // Keep classical identity
    let classical = ClassicalIdentity::from_ed25519(v1_keypair);

    // Generate new PQ identity
    let pq = PQIdentity::generate()?;

    // Create migration proof (signed by v1 key)
    let proof = MigrationProof::new(v1_keypair, &pq.public_key())?;

    Ok(HybridIdentity {
        classical,
        post_quantum: pq,
        migration_proof: Some(proof),
    })
}
```

### Session Key Derivation Changes

```rust
// v1 Key Derivation
let traffic_key = hkdf_sha256(
    shared_secret,
    b"wraith traffic key",
    32,
);

// v2 Key Derivation (new labels)
let traffic_key = hkdf_blake3(
    combined_secret,  // Hybrid secret
    b"wraith-v2-traffic-key-client-to-server",
    32,
);
```

---

## API Migration

### Builder Pattern Adoption

v2 adopts the builder pattern for complex object construction:

```rust
// v1 Style
let session = Session::new(cid, addr, secret, config)?;

// v2 Style (Builder)
let session = Session::builder()
    .connection_id(cid)
    .peer_addr(addr)
    .crypto_context(CryptoContext::from_hybrid(secret))
    .config(config)
    .wire_format(WireFormat::derive(&secret))
    .build()?;
```

### Error Type Changes

```rust
// v1 Error
pub enum Error {
    Io(std::io::Error),
    Crypto(CryptoError),
    Protocol(ProtocolError),
}

// v2 Error (more granular)
pub enum Error {
    // IO errors
    Io(std::io::Error),
    Network(NetworkError),

    // Crypto errors
    Classical(ClassicalCryptoError),
    PostQuantum(PQCryptoError),
    HybridMismatch(HybridError),

    // Protocol errors
    VersionMismatch { offered: Version, supported: Vec<Version> },
    HandshakeFailed(HandshakeError),
    FrameInvalid(FrameError),
    SessionExpired { id: ConnectionId, reason: ExpireReason },

    // Migration errors
    MigrationFailed(MigrationError),
    CompatibilityError(CompatError),
}
```

### Async API Changes

```rust
// v1 (sync with optional async)
impl Session {
    pub fn send(&self, data: &[u8]) -> Result<()>;
    pub async fn send_async(&self, data: &[u8]) -> Result<()>;
}

// v2 (async-first with sync wrapper)
impl Session {
    pub async fn send(&self, data: &[u8]) -> Result<()>;
    pub fn send_blocking(&self, data: &[u8]) -> Result<()> {
        tokio::runtime::Handle::current().block_on(self.send(data))
    }
}
```

---

## Testing Your Migration

### Migration Test Suite

```rust
#[cfg(test)]
mod migration_tests {
    use super::*;

    #[test]
    fn test_connection_id_migration() {
        let v1_cid = v1::ConnectionId::random();
        let v2_cid: v2::ConnectionId = v1_cid.into();

        // Verify migration prefix
        assert!(v2_cid.is_migrated());
        assert_eq!(v2_cid.original_v1(), Some(v1_cid));
    }

    #[tokio::test]
    async fn test_v1_v2_interop() {
        let v1_node = spawn_v1_node().await;
        let v2_node = spawn_v2_node_compat().await;

        // v2 should accept v1 connection
        let session = v2_node.connect_to(&v1_node.addr()).await?;
        assert!(session.is_v1_compat());

        // Data transfer should work
        v1_node.send(b"hello from v1").await?;
        let data = v2_node.recv().await?;
        assert_eq!(data, b"hello from v1");
    }

    #[tokio::test]
    async fn test_handshake_upgrade() {
        let v2_client = V2Client::new();
        let v2_server = V2Server::new();

        // Both v2, should use hybrid
        let session = v2_client.connect(&v2_server).await?;
        assert!(session.is_hybrid_pq());
        assert!(!session.is_v1_compat());
    }
}
```

### Compatibility Testing

```bash
# Run v1 compatibility tests
cargo test --features v1-compat migration_

# Run interop tests
cargo test --features "v1-compat,v2-native" interop_

# Benchmark migration overhead
cargo bench --features v1-compat -- migration
```

---

## Rollback Procedures

### Emergency Rollback

If critical issues are discovered post-migration:

```bash
# 1. Stop v2 nodes
systemctl stop wraith-v2

# 2. Restore v1 configuration
cp /etc/wraith/config.v1.backup /etc/wraith/config.toml

# 3. Restore v1 key material
cp -r /var/lib/wraith/keys.v1.backup /var/lib/wraith/keys

# 4. Start v1 nodes
systemctl start wraith-v1

# 5. Verify service
wraith-cli status
```

### Partial Rollback

For gradual rollback of specific components:

```rust
// Enable runtime feature flags
let config = Config::builder()
    .use_v2_crypto(false)      // Fallback to v1 crypto
    .use_v2_wire_format(false) // Fallback to v1 frames
    .use_v2_transport(true)    // Keep v2 transport
    .build()?;
```

---

## Troubleshooting

### Common Migration Issues

#### Issue: Handshake Timeout with v1 Peers

**Symptom:** v2 nodes fail to connect to v1 nodes with timeout errors.

**Solution:**
```rust
// Ensure compatibility mode is enabled
let config = Config::builder()
    .v1_compat(true)
    .v1_handshake_timeout(Duration::from_secs(30))
    .build()?;
```

#### Issue: Key Derivation Mismatch

**Symptom:** Decryption failures after handshake succeeds.

**Solution:**
```rust
// Check key derivation labels match
let key = if session.is_v1_compat() {
    derive_v1_traffic_key(&shared_secret)
} else {
    derive_v2_traffic_key(&combined_secret)
};
```

#### Issue: Frame Parsing Errors

**Symptom:** `InvalidFrameHeader` errors when receiving from v1 peers.

**Solution:**
```rust
// Use version-aware frame parser
let frame = match session.wire_version() {
    WireVersion::V1 => Frame::parse_v1(data)?,
    WireVersion::V2 => Frame::parse_v2(data, &session.format_key())?,
};
```

#### Issue: Performance Degradation in Compat Mode

**Symptom:** Lower throughput when compatibility mode is enabled.

**Explanation:** Compatibility mode requires version negotiation and potentially dual code paths.

**Mitigation:**
```rust
// Optimize for v2-only networks
let config = Config::builder()
    .v1_compat(false)
    .assume_v2_peers(true)
    .skip_version_negotiation(true)
    .build()?;
```

### Diagnostic Commands

```bash
# Check protocol version
wraith-cli session list --show-version

# Verify crypto mode
wraith-cli crypto status

# Test v1 compatibility
wraith-cli test v1-compat --target <peer>

# Analyze handshake
wraith-cli debug handshake --verbose --target <peer>
```

---

## Related Documents

- [WIRE-FORMAT-CHANGES.md](11-WRAITH-Protocol-v2-Wire-Format-Changes.md) - Detailed wire format changes
- [CRYPTO-UPGRADES.md](12-WRAITH-Protocol-v2-Crypto-Upgrades.md) - Cryptographic migration details
- [API-CHANGES.md](16-WRAITH-Protocol-v2-API-Changes.md) - Complete API changelog
- [COMPATIBILITY-MATRIX.md](15-WRAITH-Protocol-v2-Compatibility-Matrix.md) - Version compatibility details
- [TESTING-STRATEGY.md](14-WRAITH-Protocol-v2-Testing-Strategy.md) - Migration testing approach

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial migration guide |
