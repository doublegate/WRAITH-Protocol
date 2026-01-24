# WRAITH Protocol v2 Security Considerations

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Planning Document
**Authors:** WRAITH Protocol Team

---

## Table of Contents

1. [Overview](#overview)
2. [Threat Model](#threat-model)
3. [Cryptographic Security](#cryptographic-security)
4. [Protocol Security](#protocol-security)
5. [Implementation Security](#implementation-security)
6. [Operational Security](#operational-security)
7. [Migration Security](#migration-security)
8. [Security Recommendations](#security-recommendations)

---

## Overview

This document outlines security considerations specific to the WRAITH Protocol v2 migration and deployment, focusing on threats, mitigations, and best practices.

### Security Objectives

| Objective | Priority | v2 Approach |
|-----------|----------|-------------|
| Confidentiality | Critical | Hybrid encryption, per-packet FS |
| Integrity | Critical | AEAD, authenticated handshake |
| Availability | High | Multi-transport, migration |
| Traffic Analysis Resistance | High | Polymorphic format, padding |
| Post-Quantum Security | High | ML-KEM-768 hybrid |
| Forward Secrecy | High | Per-packet ratcheting |
| Identity Protection | Medium | Elligator2, probing resistance |

---

## Threat Model

### Adversary Classes

```
Adversary Capability Hierarchy:
═══════════════════════════════

Level 6: Global Passive Adversary (Nation-state)
    │    - Observe all network traffic globally
    │    - Unlimited computational resources
    │    - Quantum computing capability
    │
Level 5: Regional Network Adversary
    │    - Control regional infrastructure
    │    - Traffic correlation attacks
    │    - BGP manipulation
    │
Level 4: Active Protocol Attacker
    │    - Inject/modify packets
    │    - Replay attacks
    │    - DoS attacks
    │
Level 3: Protocol Prober
    │    - Active probing for identification
    │    - Fingerprinting attempts
    │    - Version detection
    │
Level 2: Local Network Attacker
    │    - Same LAN access
    │    - ARP spoofing
    │    - Local traffic analysis
    │
Level 1: Passive Observer
         - Observe encrypted traffic
         - No modification capability
         - Traffic metadata analysis
```

### Threat Matrix

| Threat | Adversary Level | v1 Mitigation | v2 Mitigation |
|--------|-----------------|---------------|---------------|
| Eavesdropping | 1-6 | XChaCha20 | Same + polymorphic |
| Traffic Analysis | 1-6 | Fixed padding | Continuous + timing |
| Protocol Fingerprint | 3-6 | Mimicry | Polymorphic format |
| Active Probing | 3-6 | Limited | Proof-of-knowledge |
| Replay Attack | 2-6 | Sequence numbers | Same + sliding window |
| Key Compromise | Any | Per-minute ratchet | Per-packet ratchet |
| Quantum Decryption | 6 | None | ML-KEM-768 hybrid |
| MITM Attack | 2-6 | Noise_XX | Same + PQ binding |

---

## Cryptographic Security

### Hybrid Cryptography Security

#### Security Properties

```
Hybrid KEM Security Analysis:
═════════════════════════════

Let A_classical = adversary breaking X25519
Let A_quantum = adversary breaking ML-KEM-768

Theorem: The hybrid KEM is secure if EITHER component is secure.

Proof sketch:
1. Combined_SS = BLAKE3(key || SS_classical || SS_quantum)

2. If A_classical breaks X25519:
   - SS_classical is known to adversary
   - SS_quantum is still uniformly random (ML-KEM secure)
   - Combined_SS = BLAKE3(key || known || random) = random
   - By hash preimage resistance, Combined_SS is secure

3. If A_quantum breaks ML-KEM:
   - SS_quantum is known to adversary
   - SS_classical is still uniformly random (X25519 secure)
   - Combined_SS = BLAKE3(key || random || known) = random
   - By hash preimage resistance, Combined_SS is secure

4. For Combined_SS to be compromised:
   - Must break BOTH X25519 AND ML-KEM simultaneously
   - Requires solving ECDLP AND lattice problems
   - Currently computationally infeasible

∴ Hybrid provides IND-CCA2 security under either assumption ∎
```

#### Key Combination Risks

| Risk | Description | Mitigation |
|------|-------------|------------|
| Domain Separation | Secrets from different contexts | Unique labels per use |
| Weak Hash | Hash collision exploited | BLAKE3 collision-resistant |
| Order Dependency | Combining order matters | Fixed order, length-prefixed |
| Key Reuse | Same secret for multiple purposes | Derive sub-keys via HKDF |

```rust
// Secure key combination
fn combine_hybrid_secrets(
    classical: &[u8; 32],
    post_quantum: &[u8],
) -> SharedSecret {
    let mut hasher = blake3::Hasher::new_keyed(
        b"wraith-v2-hybrid-kem-combine-ss"  // Domain separation
    );

    // Include both secrets
    hasher.update(classical);
    hasher.update(post_quantum);

    // Include lengths (prevent extension attacks)
    hasher.update(&(classical.len() as u32).to_le_bytes());
    hasher.update(&(post_quantum.len() as u32).to_le_bytes());

    SharedSecret::from(*hasher.finalize().as_bytes())
}
```

### Per-Packet Forward Secrecy

#### Security Analysis

```
Per-Packet Ratchet Security:
════════════════════════════

Chain: K0 → K1 → K2 → ... → Kn

Property 1: One-wayness
- Given Kn, cannot compute K0..Kn-1
- Relies on BLAKE3 preimage resistance

Property 2: Key Derivation Separation
- Message_Key[i] = BLAKE3(Ki, "message")
- Chain_Key[i+1] = BLAKE3(Ki, "chain")
- Different labels prevent key reuse

Property 3: Compromise Limitation
- Compromise of Ki reveals only packet i
- All K0..Ki-1 have been deleted
- Ki+1 derived before Ki deleted (atomic)

Security Claim:
- Compromising Kn at time t reveals at most 1 packet
- v1 compromise reveals up to 1,000,000 packets

Improvement Factor: 1,000,000x reduction in exposure
```

#### Ratchet Implementation Security

```rust
/// Secure ratchet implementation
pub struct PacketRatchet {
    /// Current chain key (zeroized on drop)
    chain_key: Zeroizing<[u8; 32]>,

    /// Packet number (public)
    packet_number: u64,

    /// Out-of-order key cache (limited, zeroized)
    key_cache: ZeroizingLruCache<u64, [u8; 32]>,
}

impl PacketRatchet {
    /// Advance ratchet atomically
    pub fn advance(&mut self) -> MessageKey {
        // Derive message key BEFORE advancing
        let msg_key = blake3::keyed_hash(
            &self.chain_key,
            b"wraith-v2-ratchet-message-key",
        );

        // Derive next chain key
        let next_chain = blake3::keyed_hash(
            &self.chain_key,
            b"wraith-v2-ratchet-chain-next",
        );

        // CRITICAL: Zeroize current key before updating
        self.chain_key.zeroize();
        self.chain_key = Zeroizing::new(*next_chain.as_bytes());

        self.packet_number += 1;

        MessageKey::from(*msg_key.as_bytes())
    }
}

impl Drop for PacketRatchet {
    fn drop(&mut self) {
        // Explicitly zeroize all key material
        self.chain_key.zeroize();
        // key_cache auto-zeroizes via ZeroizingLruCache
    }
}
```

---

## Protocol Security

### Handshake Security

#### Attack Resistance

| Attack | Mechanism | Protection |
|--------|-----------|------------|
| Replay | Resend handshake messages | Nonce/timestamp + state machine |
| Downgrade | Force v1 protocol | Version pinning option |
| Identity Misbinding | Swap identities | Transcript binding |
| Unknown Key Share | Attacker key substitution | Full transcript hash |
| Key Compromise Impersonation | Use victim's key | Hybrid binding |

```rust
/// Secure handshake with attack mitigations
pub struct SecureHandshake {
    /// Handshake state machine
    state: HandshakeState,

    /// Full transcript for binding
    transcript: TranscriptHash,

    /// Nonces for replay prevention
    nonces: NonceSet,

    /// Start time for timing attacks
    start_time: Instant,
}

impl SecureHandshake {
    pub fn process_message(
        &mut self,
        message: &HandshakeMessage,
    ) -> Result<Option<HandshakeMessage>> {
        // Verify state machine transition
        self.verify_state_transition(message.msg_type())?;

        // Check nonce freshness
        self.verify_nonce_fresh(&message.nonce)?;

        // Update transcript
        self.transcript.update(&message.encode());

        // Verify timing bounds (prevent timing analysis)
        self.verify_timing()?;

        // Process based on current state
        match self.state {
            HandshakeState::WaitingInit => self.process_init(message),
            HandshakeState::WaitingResponse => self.process_response(message),
            HandshakeState::WaitingComplete => self.process_complete(message),
            _ => Err(HandshakeError::InvalidState),
        }
    }

    fn verify_timing(&self) -> Result<()> {
        let elapsed = self.start_time.elapsed();

        // Handshake should complete within reasonable time
        if elapsed > MAX_HANDSHAKE_DURATION {
            return Err(HandshakeError::Timeout);
        }

        Ok(())
    }
}
```

### Probing Resistance

#### Design

```
Probing Resistance Mechanism:
═════════════════════════════

1. Require proof-of-knowledge of server public key
   - Client must prove they know S_pub before server responds
   - Prevents enumeration of valid servers

2. No response to invalid probes
   - Invalid packets get no response (silent drop)
   - Prevents oracle attacks

3. Timing uniformity
   - All operations take constant time
   - Prevents timing-based probing

Probe Format:
┌─────────────────────────────────────────────────────┐
│ Random Prefix (8 bytes) - Appears random           │
├─────────────────────────────────────────────────────┤
│ Proof = BLAKE3(server_pubkey || timestamp || nonce)│
│         (32 bytes)                                  │
├─────────────────────────────────────────────────────┤
│ Encrypted Payload (variable)                       │
└─────────────────────────────────────────────────────┘

Server Verification:
1. Extract proof from expected position
2. Recompute expected proof using own public key
3. Constant-time comparison
4. If match: process, else: silent drop
```

```rust
/// Probing-resistant server
pub struct ProbingResistantServer {
    /// Server keypair
    keypair: HybridKeyPair,

    /// Recent nonces (replay prevention)
    seen_nonces: BloomFilter,

    /// Timing randomization
    timing_noise: Distribution,
}

impl ProbingResistantServer {
    pub async fn handle_packet(&self, packet: &[u8]) -> Option<Vec<u8>> {
        // Add timing noise
        let delay = self.timing_noise.sample();
        tokio::time::sleep(delay).await;

        // Verify proof-of-knowledge
        if !self.verify_proof(packet) {
            // CRITICAL: No response to invalid probes
            return None;
        }

        // Check replay
        let nonce = self.extract_nonce(packet)?;
        if self.seen_nonces.contains(&nonce) {
            return None;  // Silent drop replays
        }
        self.seen_nonces.insert(&nonce);

        // Process valid packet
        self.process_packet(packet).await
    }

    fn verify_proof(&self, packet: &[u8]) -> bool {
        if packet.len() < PROOF_SIZE {
            return false;
        }

        let received_proof = &packet[8..40];  // Proof position

        // Recompute expected proof
        let expected = self.compute_expected_proof(packet);

        // CRITICAL: Constant-time comparison
        constant_time_eq(received_proof, &expected)
    }
}
```

---

## Implementation Security

### Memory Security

| Risk | Mitigation |
|------|------------|
| Key in swap | mlock() on key memory |
| Key in core dump | Disable core dumps |
| Key in memory | Zeroize on drop |
| Buffer overflow | Rust memory safety |
| Use-after-free | Rust ownership |

```rust
/// Secure key storage
pub struct SecureKey {
    /// Locked, non-swappable memory
    inner: LockedBox<[u8; 32]>,
}

impl SecureKey {
    pub fn new(key: [u8; 32]) -> Result<Self> {
        // Allocate in locked memory
        let mut locked = LockedBox::new([0u8; 32])?;
        locked.copy_from_slice(&key);

        // Zeroize source
        let mut key = key;
        key.zeroize();

        Ok(Self { inner: locked })
    }

    pub fn expose(&self) -> &[u8; 32] {
        &self.inner
    }
}

impl Drop for SecureKey {
    fn drop(&mut self) {
        // LockedBox handles zeroization
        // Memory unlocking happens automatically
    }
}

impl Zeroize for SecureKey {
    fn zeroize(&mut self) {
        self.inner.zeroize();
    }
}
```

### Constant-Time Operations

```rust
/// Constant-time utilities
pub mod constant_time {
    /// Constant-time byte comparison
    pub fn eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }

        result == 0
    }

    /// Constant-time conditional select
    pub fn select(condition: bool, a: &[u8], b: &[u8]) -> Vec<u8> {
        debug_assert_eq!(a.len(), b.len());

        let mask = if condition { 0xFF } else { 0x00 };
        let not_mask = !mask;

        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x & mask) | (y & not_mask))
            .collect()
    }

    /// Constant-time lookup in table
    pub fn table_lookup(table: &[[u8; 32]], index: usize) -> [u8; 32] {
        let mut result = [0u8; 32];

        for (i, entry) in table.iter().enumerate() {
            let mask = constant_time_eq_usize(i, index);
            for j in 0..32 {
                result[j] |= entry[j] & mask;
            }
        }

        result
    }

    fn constant_time_eq_usize(a: usize, b: usize) -> u8 {
        let diff = a ^ b;
        let is_zero = (diff | diff.wrapping_neg()) >> (usize::BITS - 1);
        (1 - is_zero) as u8 * 0xFF
    }
}
```

### Side-Channel Mitigations

| Channel | Risk | Mitigation |
|---------|------|------------|
| Timing | Key-dependent timing | Constant-time code |
| Cache | Key-dependent access | No table lookups with secrets |
| Power | Key-dependent power | Balanced operations |
| EM | Key leakage via emissions | Hardware shielding |

---

## Operational Security

### Deployment Security

```yaml
# Secure deployment checklist
deployment:
  pre_deployment:
    - Verify binary signatures
    - Check dependency hashes
    - Review configuration
    - Test in staging environment

  runtime:
    - Enable ASLR
    - Enable stack canaries
    - Disable core dumps
    - Use seccomp filters
    - Run as non-root user
    - Isolate with containers/VMs

  monitoring:
    - Log security events
    - Monitor for anomalies
    - Alert on suspicious patterns
    - Rotate logs securely

  key_management:
    - Generate keys securely
    - Store keys encrypted
    - Rotate keys regularly
    - Secure key backup
    - Destroy old keys
```

### Key Lifecycle

```
Key Lifecycle Management:
═════════════════════════

Generation        Storage          Usage            Rotation         Destruction
    │                │                │                │                │
    ▼                ▼                ▼                ▼                ▼
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
│ Secure  │───►│Encrypted│───►│ Locked  │───►│ New Key │───►│ Zeroize │
│  RNG    │    │  Store  │    │ Memory  │    │Generated│    │  & Wipe │
└─────────┘    └─────────┘    └─────────┘    └─────────┘    └─────────┘

Identity Keys:
- Rotate: Annually or on compromise suspicion
- Backup: Encrypted, geographically distributed

Session Keys:
- Derived fresh per session
- Never stored persistently
- Destroyed on session close

Ephemeral Keys:
- Generated per handshake
- Never stored
- Destroyed after use
```

---

## Migration Security

### Secure Migration Practices

| Phase | Security Concern | Mitigation |
|-------|------------------|------------|
| Pre-Migration | Key exposure during export | Encrypted export |
| Migration | Downgrade attacks | Version pinning |
| Post-Migration | v1 compatibility risks | Limited compat window |
| Cleanup | Old key material | Secure deletion |

### Identity Migration

```rust
/// Secure identity migration
pub async fn migrate_identity(
    v1_identity: &V1Identity,
    v1_passphrase: &str,
) -> Result<V2Identity> {
    // Verify v1 identity ownership
    v1_identity.verify_passphrase(v1_passphrase)?;

    // Generate new PQ keypair
    let pq_keypair = MlDsa65::generate();

    // Create binding proof
    // (proves ownership of both classical and PQ keys)
    let binding_message = format!(
        "WRAITH-v2-identity-migration:{}:{}:{}",
        hex::encode(v1_identity.public_key.as_bytes()),
        hex::encode(pq_keypair.public_key.as_bytes()),
        chrono::Utc::now().timestamp(),
    );

    let classical_sig = v1_identity.sign(binding_message.as_bytes());
    let pq_sig = pq_keypair.sign(binding_message.as_bytes());

    let v2_identity = V2Identity {
        classical: v1_identity.clone(),
        post_quantum: Some(pq_keypair),
        binding: Some(IdentityBinding {
            message: binding_message,
            classical_signature: classical_sig,
            pq_signature: pq_sig,
        }),
    };

    // Securely store new identity
    v2_identity.store_encrypted(v1_passphrase)?;

    Ok(v2_identity)
}
```

### Compatibility Mode Risks

```
Compatibility Mode Security Trade-offs:
═══════════════════════════════════════

Enabled (v1 + v2):
├── Pro: Gradual migration possible
├── Pro: No flag day required
├── Con: v1 weaknesses still exploitable
├── Con: Downgrade attack surface
└── Con: Larger code surface

Disabled (v2 only):
├── Pro: Maximum security
├── Pro: Smaller attack surface
├── Con: Breaks v1 clients
└── Con: Requires coordinated upgrade

Recommendation:
1. Enable compat mode initially
2. Monitor v1 usage metrics
3. Set deprecation timeline
4. Disable after migration period
5. Remove v1 code entirely
```

---

## Security Recommendations

### Deployment Recommendations

| Priority | Recommendation | Rationale |
|----------|---------------|-----------|
| Critical | Enable hybrid crypto | Post-quantum protection |
| Critical | Use per-packet ratchet | Maximum forward secrecy |
| High | Enable probing resistance | Prevent enumeration |
| High | Use polymorphic format | Traffic analysis resistance |
| High | Minimize compat window | Reduce attack surface |
| Medium | Enable all obfuscation | Maximum anonymity |
| Medium | Use multi-transport | Availability |
| Low | Enable FEC | Reliability in lossy networks |

### Configuration Best Practices

```rust
/// Recommended secure configuration
pub fn secure_default_config() -> Config {
    Config::builder()
        // Crypto settings
        .crypto_mode(CryptoMode::HybridPQ)
        .ratchet_mode(RatchetMode::PerPacket)
        .signature_mode(SignatureMode::HybridOptional)

        // Wire format
        .wire_format(WireFormat::Polymorphic)
        .padding_strategy(PaddingStrategy::Continuous {
            distribution: Distribution::Uniform { min: 0, max: 128 },
        })
        .timing_strategy(TimingStrategy::AdaptiveMarkov)

        // Protocol security
        .probing_resistance(true)
        .version_downgrade_protection(true)
        .replay_window_size(1024)

        // Compatibility (disable after migration)
        .v1_compat(false)

        // Operational
        .key_rotation_interval(Duration::from_days(90))
        .session_timeout(Duration::from_hours(24))
        .max_handshake_duration(Duration::from_secs(30))

        .build()
}
```

### Audit Recommendations

| Area | Frequency | Focus |
|------|-----------|-------|
| Crypto code | Every release | Correctness, timing |
| Key handling | Quarterly | Lifecycle, storage |
| Protocol logic | Every release | State machine, edge cases |
| Dependencies | Weekly | CVE monitoring |
| Configuration | Monthly | Drift detection |

---

## Related Documents

- [Security Analysis](06-WRAITH-Protocol-v2-Security-Analysis.md) - Detailed security analysis
- [Crypto Upgrades](12-WRAITH-Protocol-v2-Crypto-Upgrades.md) - Cryptographic changes
- [Migration Guide](09-WRAITH-Protocol-v2-Migration-Guide.md) - Migration instructions
- [Specification](01-WRAITH-Protocol-v2-Specification.md) - Protocol specification

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial security considerations document |
