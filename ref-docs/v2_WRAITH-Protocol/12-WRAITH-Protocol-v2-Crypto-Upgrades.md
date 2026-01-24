# WRAITH Protocol v2 Cryptographic Upgrades

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Planning Document
**Authors:** WRAITH Protocol Team

---

## Table of Contents

1. [Overview](#overview)
2. [Hybrid Key Exchange](#hybrid-key-exchange)
3. [Post-Quantum Signatures](#post-quantum-signatures)
4. [Key Derivation Changes](#key-derivation-changes)
5. [Per-Packet Forward Secrecy](#per-packet-forward-secrecy)
6. [AEAD Changes](#aead-changes)
7. [Handshake Protocol](#handshake-protocol)
8. [Key Management](#key-management)
9. [Security Analysis](#security-analysis)
10. [Migration Guide](#migration-guide)

---

## Overview

WRAITH Protocol v2 introduces significant cryptographic upgrades to provide post-quantum security while maintaining classical security guarantees. The hybrid approach ensures security even if either classical or post-quantum algorithms are compromised.

### Cryptographic Summary

| Component | v1 | v2 | Security Level |
|-----------|----|----|----------------|
| Key Exchange | X25519 | X25519 + ML-KEM-768 | 128-bit classical + 128-bit PQ |
| Signatures | Ed25519 | Ed25519 + ML-DSA-65 (optional) | 128-bit classical + 128-bit PQ |
| AEAD | XChaCha20-Poly1305 | XChaCha20-Poly1305 | 256-bit |
| Hash | BLAKE3 | BLAKE3 | 256-bit |
| KDF | HKDF-SHA256 | HKDF-BLAKE3 | 256-bit |
| Forward Secrecy | Per-minute/1M packets | Per-packet | Granular |

### Security Targets

- **Classical Security:** 128-bit against classical computers
- **Post-Quantum Security:** 128-bit against quantum computers (NIST Level 1)
- **Forward Secrecy:** Per-packet granularity
- **Key Compromise Impersonation:** Prevented by hybrid design
- **Harvest-Now-Decrypt-Later:** Mitigated by PQ cryptography

---

## Hybrid Key Exchange

### Design Rationale

The hybrid approach combines X25519 (elliptic curve Diffie-Hellman) with ML-KEM-768 (post-quantum lattice-based KEM):

1. **Defense in Depth:** Security maintained if either algorithm breaks
2. **Proven Classical:** X25519 has extensive real-world deployment
3. **Standards Compliance:** ML-KEM-768 is NIST FIPS 203
4. **Performance:** Hybrid adds ~0.5ms to handshake

### Implementation

```rust
use ml_kem::{MlKem768, Encapsulate, Decapsulate};
use x25519_dalek::{EphemeralSecret, PublicKey};

/// Hybrid public key for key exchange
#[derive(Clone)]
pub struct HybridPublicKey {
    pub classical: x25519_dalek::PublicKey,
    pub post_quantum: ml_kem::EncapsulationKey<MlKem768>,
}

/// Hybrid secret key for key exchange
pub struct HybridSecretKey {
    classical: x25519_dalek::StaticSecret,
    post_quantum: ml_kem::DecapsulationKey<MlKem768>,
}

/// Hybrid ciphertext from encapsulation
#[derive(Clone)]
pub struct HybridCiphertext {
    pub classical: [u8; 32],          // X25519 public key
    pub post_quantum: ml_kem::Ciphertext<MlKem768>,  // ML-KEM ciphertext
}

impl HybridSecretKey {
    /// Generate new hybrid keypair
    pub fn generate() -> (Self, HybridPublicKey) {
        // Generate X25519 keypair
        let classical_secret = x25519_dalek::StaticSecret::random_from_rng(
            &mut rand::thread_rng()
        );
        let classical_public = x25519_dalek::PublicKey::from(&classical_secret);

        // Generate ML-KEM-768 keypair
        let (dk, ek) = MlKem768::generate(&mut rand::thread_rng());

        (
            Self {
                classical: classical_secret,
                post_quantum: dk,
            },
            HybridPublicKey {
                classical: classical_public,
                post_quantum: ek,
            },
        )
    }

    /// Decapsulate hybrid ciphertext to recover shared secret
    pub fn decapsulate(&self, ct: &HybridCiphertext) -> SharedSecret {
        // X25519 key agreement
        let classical_ss = self.classical.diffie_hellman(
            &x25519_dalek::PublicKey::from(ct.classical)
        );

        // ML-KEM decapsulation
        let pq_ss = self.post_quantum.decapsulate(&ct.post_quantum)
            .expect("ML-KEM decapsulation should not fail with valid ciphertext");

        // Combine shared secrets
        combine_shared_secrets(classical_ss.as_bytes(), pq_ss.as_ref())
    }
}

impl HybridPublicKey {
    /// Encapsulate to peer's public key
    pub fn encapsulate(&self) -> (SharedSecret, HybridCiphertext) {
        // X25519: Generate ephemeral and compute DH
        let ephemeral = x25519_dalek::EphemeralSecret::random_from_rng(
            &mut rand::thread_rng()
        );
        let ephemeral_public = x25519_dalek::PublicKey::from(&ephemeral);
        let classical_ss = ephemeral.diffie_hellman(&self.classical);

        // ML-KEM encapsulation
        let (pq_ct, pq_ss) = self.post_quantum.encapsulate(&mut rand::thread_rng());

        // Combine shared secrets
        let combined = combine_shared_secrets(
            classical_ss.as_bytes(),
            pq_ss.as_ref(),
        );

        (
            combined,
            HybridCiphertext {
                classical: ephemeral_public.to_bytes(),
                post_quantum: pq_ct,
            },
        )
    }
}

/// Combine classical and post-quantum shared secrets
fn combine_shared_secrets(classical: &[u8; 32], post_quantum: &[u8]) -> SharedSecret {
    // Domain-separated combination using BLAKE3
    let mut hasher = blake3::Hasher::new_keyed(
        b"wraith-hybrid-kem-v2-combine-ss"
    );

    // Include both shared secrets
    hasher.update(classical);
    hasher.update(post_quantum);

    // Include lengths for domain separation
    hasher.update(&(classical.len() as u32).to_le_bytes());
    hasher.update(&(post_quantum.len() as u32).to_le_bytes());

    SharedSecret::from(*hasher.finalize().as_bytes())
}
```

### Hybrid KEM Security Properties

```
Security Proof Sketch:
─────────────────────

Let SS_c = X25519 shared secret
Let SS_pq = ML-KEM-768 shared secret
Let SS_combined = BLAKE3("wraith-hybrid-kem-v2-combine-ss", SS_c || SS_pq)

Claim: SS_combined is secure if EITHER SS_c OR SS_pq is secure.

Proof:
1. If X25519 is secure:
   - SS_c is uniformly random to adversary
   - SS_combined = BLAKE3(random || SS_pq) is random (hash preimage resistance)

2. If ML-KEM-768 is secure:
   - SS_pq is uniformly random to adversary
   - SS_combined = BLAKE3(SS_c || random) is random (hash preimage resistance)

3. Both broken simultaneously:
   - Adversary must break ECDLP (X25519) AND lattice problem (ML-KEM)
   - Considered computationally infeasible

∴ Hybrid provides security if either component is secure ∎
```

---

## Post-Quantum Signatures

### Optional ML-DSA-65 Integration

While identity verification continues to use Ed25519 for performance, v2 adds optional ML-DSA-65 (FIPS 204) for post-quantum identity binding:

```rust
use ml_dsa::{MlDsa65, SigningKey, VerifyingKey};

/// Hybrid identity for long-term authentication
pub struct HybridIdentity {
    /// Classical Ed25519 identity (primary)
    pub classical: Ed25519Identity,

    /// Post-quantum ML-DSA-65 identity (optional)
    pub post_quantum: Option<MlDsaIdentity>,

    /// Binding proof (classical signs PQ public key)
    pub binding_signature: Option<Ed25519Signature>,
}

pub struct Ed25519Identity {
    pub signing_key: ed25519_dalek::SigningKey,
    pub verifying_key: ed25519_dalek::VerifyingKey,
}

pub struct MlDsaIdentity {
    signing_key: ml_dsa::SigningKey<MlDsa65>,
    verifying_key: ml_dsa::VerifyingKey<MlDsa65>,
}

impl HybridIdentity {
    /// Generate new hybrid identity
    pub fn generate() -> Self {
        // Generate Ed25519 identity
        let classical = Ed25519Identity::generate();

        // Generate ML-DSA-65 identity
        let (pq_sk, pq_vk) = MlDsa65::generate(&mut rand::thread_rng());
        let post_quantum = MlDsaIdentity {
            signing_key: pq_sk,
            verifying_key: pq_vk,
        };

        // Bind PQ identity to classical identity
        let binding_message = format!(
            "WRAITH-v2-identity-binding:{}",
            hex::encode(post_quantum.verifying_key.as_bytes())
        );
        let binding_signature = classical.signing_key.sign(
            binding_message.as_bytes()
        );

        Self {
            classical,
            post_quantum: Some(post_quantum),
            binding_signature: Some(binding_signature),
        }
    }

    /// Sign message with hybrid identity
    pub fn sign(&self, message: &[u8]) -> HybridSignature {
        let classical_sig = self.classical.signing_key.sign(message);

        let pq_sig = self.post_quantum.as_ref().map(|pq| {
            pq.signing_key.sign(&mut rand::thread_rng(), message)
        });

        HybridSignature {
            classical: classical_sig,
            post_quantum: pq_sig,
        }
    }
}

/// Hybrid signature
pub struct HybridSignature {
    pub classical: ed25519_dalek::Signature,
    pub post_quantum: Option<ml_dsa::Signature<MlDsa65>>,
}

impl HybridSignature {
    /// Verify hybrid signature (both must verify if PQ present)
    pub fn verify(
        &self,
        message: &[u8],
        identity: &HybridIdentity,
    ) -> Result<(), SignatureError> {
        // Always verify classical
        identity.classical.verifying_key.verify(message, &self.classical)?;

        // Verify PQ if present
        if let (Some(pq_sig), Some(pq_id)) = (&self.post_quantum, &identity.post_quantum) {
            pq_id.verifying_key.verify(message, pq_sig)?;
        }

        Ok(())
    }
}
```

### Signature Size Comparison

| Algorithm | Public Key | Signature | Security |
|-----------|------------|-----------|----------|
| Ed25519 | 32 bytes | 64 bytes | 128-bit classical |
| ML-DSA-65 | 1,952 bytes | 3,309 bytes | 128-bit PQ |
| Hybrid | 1,984 bytes | 3,373 bytes | Both |

---

## Key Derivation Changes

### HKDF-BLAKE3 Migration

v2 replaces HKDF-SHA256 with HKDF-BLAKE3 for all key derivation:

```rust
/// HKDF using BLAKE3 as the underlying hash
pub struct HkdfBlake3;

impl HkdfBlake3 {
    /// Extract pseudorandom key from input key material
    pub fn extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
        blake3::keyed_hash(
            blake3::hash(salt).as_bytes(),
            ikm,
        ).into()
    }

    /// Expand pseudorandom key to derived key material
    pub fn expand(prk: &[u8; 32], info: &[u8], output: &mut [u8]) {
        let mut counter = 1u8;
        let mut prev = [0u8; 32];
        let mut offset = 0;

        while offset < output.len() {
            let mut hasher = blake3::Hasher::new_keyed(prk);
            if counter > 1 {
                hasher.update(&prev);
            }
            hasher.update(info);
            hasher.update(&[counter]);

            let block = hasher.finalize();
            prev = *block.as_bytes();

            let to_copy = (output.len() - offset).min(32);
            output[offset..offset + to_copy].copy_from_slice(&prev[..to_copy]);
            offset += to_copy;
            counter += 1;
        }
    }

    /// Combined extract-and-expand
    pub fn derive(
        salt: &[u8],
        ikm: &[u8],
        info: &[u8],
        output: &mut [u8],
    ) {
        let prk = Self::extract(salt, ikm);
        Self::expand(&prk, info, output);
    }
}
```

### New Key Derivation Labels

v2 uses new domain-separated labels for all derived keys:

```rust
/// Key derivation labels for v2
pub mod labels {
    // Handshake keys
    pub const HANDSHAKE_SECRET: &[u8] = b"wraith-v2-handshake-secret";
    pub const HANDSHAKE_KEY_CLIENT: &[u8] = b"wraith-v2-handshake-key-client";
    pub const HANDSHAKE_KEY_SERVER: &[u8] = b"wraith-v2-handshake-key-server";

    // Traffic keys
    pub const TRAFFIC_SECRET: &[u8] = b"wraith-v2-traffic-secret";
    pub const TRAFFIC_KEY_CLIENT_TO_SERVER: &[u8] = b"wraith-v2-traffic-key-c2s";
    pub const TRAFFIC_KEY_SERVER_TO_CLIENT: &[u8] = b"wraith-v2-traffic-key-s2c";

    // Ratchet keys
    pub const RATCHET_CHAIN: &[u8] = b"wraith-v2-ratchet-chain";
    pub const RATCHET_MESSAGE: &[u8] = b"wraith-v2-ratchet-message";

    // Wire format keys
    pub const FORMAT_KEY: &[u8] = b"wraith-v2-format-key";
    pub const FORMAT_POSITIONS: &[u8] = b"wraith-v2-format-positions";
    pub const FORMAT_XOR_MASK: &[u8] = b"wraith-v2-format-xor-mask";

    // Group keys (TreeKEM)
    pub const GROUP_SECRET: &[u8] = b"wraith-v2-group-secret";
    pub const GROUP_APPLICATION: &[u8] = b"wraith-v2-group-application";

    // Hybrid combination
    pub const HYBRID_COMBINE: &[u8] = b"wraith-v2-hybrid-combine";
}

/// Derive session keys from combined secret
pub fn derive_session_keys(
    combined_secret: &[u8; 32],
    transcript_hash: &[u8; 32],
) -> SessionKeys {
    let mut keys = SessionKeys::default();

    // Derive traffic secret
    let mut traffic_secret = [0u8; 32];
    HkdfBlake3::derive(
        transcript_hash,
        combined_secret,
        labels::TRAFFIC_SECRET,
        &mut traffic_secret,
    );

    // Derive directional keys
    HkdfBlake3::derive(
        &traffic_secret,
        &[],
        labels::TRAFFIC_KEY_CLIENT_TO_SERVER,
        &mut keys.client_to_server,
    );

    HkdfBlake3::derive(
        &traffic_secret,
        &[],
        labels::TRAFFIC_KEY_SERVER_TO_CLIENT,
        &mut keys.server_to_client,
    );

    // Derive format key
    HkdfBlake3::derive(
        &traffic_secret,
        &[],
        labels::FORMAT_KEY,
        &mut keys.format_key,
    );

    keys
}
```

### v1 to v2 Label Mapping

| Purpose | v1 Label | v2 Label |
|---------|----------|----------|
| Traffic Key | `"wraith traffic key"` | `"wraith-v2-traffic-key-c2s"` |
| Ratchet | `"wraith ratchet chain"` | `"wraith-v2-ratchet-chain"` |
| Format | N/A (static) | `"wraith-v2-format-key"` |

---

## Per-Packet Forward Secrecy

### Ratchet Design

v2 implements per-packet forward secrecy through a hash-based ratchet:

```rust
/// Per-packet ratchet for forward secrecy
pub struct PacketRatchet {
    /// Current chain key
    chain_key: [u8; 32],

    /// Current packet number
    packet_number: u64,

    /// Cached message keys for out-of-order delivery
    key_cache: LruCache<u64, [u8; 32]>,

    /// Maximum packets to cache
    max_cache_size: usize,
}

impl PacketRatchet {
    /// Create new ratchet from initial secret
    pub fn new(initial_secret: &[u8; 32]) -> Self {
        let mut chain_key = [0u8; 32];
        HkdfBlake3::derive(
            &[],
            initial_secret,
            labels::RATCHET_CHAIN,
            &mut chain_key,
        );

        Self {
            chain_key,
            packet_number: 0,
            key_cache: LruCache::new(1024),
            max_cache_size: 1024,
        }
    }

    /// Derive key for next packet (sending)
    pub fn next_send_key(&mut self) -> (u64, [u8; 32]) {
        let pn = self.packet_number;
        let key = self.derive_message_key();
        self.advance_chain();
        self.packet_number += 1;
        (pn, key)
    }

    /// Derive key for received packet (may be out of order)
    pub fn key_for_packet(&mut self, packet_number: u64) -> Result<[u8; 32], RatchetError> {
        // Check if we've already advanced past this packet
        if packet_number < self.packet_number.saturating_sub(self.max_cache_size as u64) {
            return Err(RatchetError::PacketTooOld);
        }

        // Check cache first
        if let Some(key) = self.key_cache.get(&packet_number) {
            return Ok(*key);
        }

        // Need to advance to this packet
        while self.packet_number <= packet_number {
            let key = self.derive_message_key();

            // Cache key if not the target (for out-of-order packets)
            if self.packet_number < packet_number {
                self.key_cache.put(self.packet_number, key);
            }

            self.advance_chain();

            if self.packet_number - 1 == packet_number {
                return Ok(key);
            }

            self.packet_number += 1;
        }

        Err(RatchetError::PacketNotFound)
    }

    /// Derive message key from current chain key
    fn derive_message_key(&self) -> [u8; 32] {
        blake3::keyed_hash(
            &self.chain_key,
            labels::RATCHET_MESSAGE,
        ).into()
    }

    /// Advance chain key (deleting old key material)
    fn advance_chain(&mut self) {
        let new_chain = blake3::keyed_hash(
            &self.chain_key,
            labels::RATCHET_CHAIN,
        );

        // Securely zero old chain key
        self.chain_key.zeroize();
        self.chain_key = *new_chain.as_bytes();
    }
}
```

### Forward Secrecy Comparison

```
v1 Ratchet (per-minute or per-1M packets):
─────────────────────────────────────────
Time:    t0          t1 (1 min)      t2 (2 min)
         │           │               │
Keys:    K0 ────────►K1 ────────────►K2
         │           │               │
Packets: p0..p999999 p1000000..      ...

Compromise at t1: All packets p0..p999999 exposed

v2 Ratchet (per-packet):
────────────────────────
Packet:  p0    p1    p2    p3    p4    ...
         │     │     │     │     │
Keys:    K0───►K1───►K2───►K3───►K4───►...
         │     │     │     │     │
         ▼     ▼     ▼     ▼     ▼
         M0    M1    M2    M3    M4    (message keys)

Compromise of Kn: Only packet pn exposed
                  All previous keys deleted

Security Improvement:
- v1: Up to 1,000,000 packets exposed per compromise
- v2: Maximum 1 packet exposed per compromise
- Improvement: 1,000,000x reduction in exposure window
```

---

## AEAD Changes

### XChaCha20-Poly1305 Retained

v2 retains XChaCha20-Poly1305 as the AEAD cipher due to:

1. **256-bit Security:** Sufficient for post-quantum hybrid
2. **192-bit Nonce:** Safe for random nonce generation
3. **Performance:** Hardware-optimized on modern CPUs
4. **Simplicity:** Well-analyzed, minimal attack surface

### Nonce Generation

```rust
/// Nonce generation for AEAD
pub struct NonceGenerator {
    /// Random prefix (12 bytes)
    prefix: [u8; 12],

    /// Counter (8 bytes)
    counter: u64,

    /// Session-derived mask
    mask: [u8; 24],
}

impl NonceGenerator {
    /// Create from session secret
    pub fn new(session_secret: &[u8; 32]) -> Self {
        let mut prefix = [0u8; 12];
        let mut mask = [0u8; 24];

        HkdfBlake3::derive(
            &[],
            session_secret,
            b"wraith-v2-nonce-prefix",
            &mut prefix,
        );

        HkdfBlake3::derive(
            &[],
            session_secret,
            b"wraith-v2-nonce-mask",
            &mut mask,
        );

        Self {
            prefix,
            counter: 0,
            mask,
        }
    }

    /// Generate nonce for packet
    pub fn for_packet(&self, packet_number: u64) -> [u8; 24] {
        let mut nonce = [0u8; 24];

        // First 12 bytes: random prefix
        nonce[..12].copy_from_slice(&self.prefix);

        // Bytes 12-19: XOR of counter and packet number
        let combined = self.counter ^ packet_number;
        nonce[12..20].copy_from_slice(&combined.to_le_bytes());

        // Bytes 20-24: padding
        nonce[20..24].fill(0);

        // Apply mask
        for i in 0..24 {
            nonce[i] ^= self.mask[i];
        }

        nonce
    }
}
```

---

## Handshake Protocol

### Hybrid Noise_XX + ML-KEM

v2 extends the Noise_XX pattern with ML-KEM encapsulation:

```
Hybrid Handshake Protocol:
═══════════════════════════

Initiator (I)                                    Responder (R)
─────────────────                                ─────────────────
Generate:                                        Has:
  - e_c (X25519 ephemeral)                        - s_c, S_c (X25519 static)
  - e_pq (ML-KEM ephemeral)                       - s_pq, S_pq (ML-KEM static)

Message 1: I → R
┌────────────────────────────────────────────────────────────────┐
│  e_c.public                                                    │
│  e_pq.encapsulation_key                                        │
│  encrypt(payload1)   // with initial key                       │
└────────────────────────────────────────────────────────────────┘

                                                 Process:
                                                   - DH(e_c, s_c) → ss1
                                                   - Encapsulate(e_pq) → (ss_pq1, ct_pq1)
                                                   - Derive keys

Message 2: R → I
┌────────────────────────────────────────────────────────────────┐
│  E_c.public          // R's X25519 ephemeral                   │
│  E_pq.encapsulation_key  // R's ML-KEM ephemeral               │
│  ct_pq1              // ML-KEM ciphertext for I's key          │
│  S_c.public          // R's static, encrypted                  │
│  S_pq.encapsulation_key  // R's static ML-KEM, encrypted       │
│  encrypt(payload2)                                             │
└────────────────────────────────────────────────────────────────┘

Process on I:
  - DH(e_c, E_c) → ss2
  - DH(e_c, S_c) → ss3
  - Decapsulate(ct_pq1) → ss_pq1
  - Encapsulate(E_pq) → (ss_pq2, ct_pq2)
  - Encapsulate(S_pq) → (ss_pq3, ct_pq3)

Message 3: I → R
┌────────────────────────────────────────────────────────────────┐
│  ct_pq2              // ML-KEM ciphertext for R's ephemeral    │
│  ct_pq3              // ML-KEM ciphertext for R's static       │
│  s_c.public          // I's static, encrypted                  │
│  s_pq.encapsulation_key  // I's static ML-KEM, encrypted       │
│  encrypt(payload3)                                             │
└────────────────────────────────────────────────────────────────┘

Final Key Derivation:
  combined = BLAKE3(
    "wraith-v2-handshake-combined",
    ss1 || ss2 || ss3 ||           // X25519 secrets
    ss_pq1 || ss_pq2 || ss_pq3 ||  // ML-KEM secrets
    transcript_hash
  )
```

### Implementation

```rust
/// Hybrid handshake state machine
pub struct HybridHandshake {
    /// Classical Noise state
    noise: snow::HandshakeState,

    /// ML-KEM ephemeral keypair
    pq_ephemeral: MlKem768KeyPair,

    /// ML-KEM shared secrets collected
    pq_secrets: Vec<[u8; 32]>,

    /// Role in handshake
    role: HandshakeRole,

    /// Handshake transcript
    transcript: Vec<u8>,
}

impl HybridHandshake {
    pub fn new_initiator(
        static_key: &HybridSecretKey,
        peer_static_pq: Option<&ml_kem::EncapsulationKey<MlKem768>>,
    ) -> Result<Self> {
        let params = snow::params::NoiseParams::new(
            "Noise_XX_25519_ChaChaPoly_BLAKE2s".parse()?
        );

        let noise = snow::Builder::new(params)
            .local_private_key(&static_key.classical.to_bytes())
            .build_initiator()?;

        let pq_ephemeral = MlKem768KeyPair::generate();

        Ok(Self {
            noise,
            pq_ephemeral,
            pq_secrets: Vec::new(),
            role: HandshakeRole::Initiator,
            transcript: Vec::new(),
        })
    }

    pub async fn execute<T: AsyncReadWrite>(
        &mut self,
        stream: &mut T,
    ) -> Result<SessionSecrets> {
        match self.role {
            HandshakeRole::Initiator => self.execute_initiator(stream).await,
            HandshakeRole::Responder => self.execute_responder(stream).await,
        }
    }

    async fn execute_initiator<T: AsyncReadWrite>(
        &mut self,
        stream: &mut T,
    ) -> Result<SessionSecrets> {
        // Message 1: Send ephemeral keys
        let mut msg1 = vec![0u8; 65536];
        let len = self.noise.write_message(&[], &mut msg1)?;
        msg1.truncate(len);

        // Append ML-KEM encapsulation key
        msg1.extend_from_slice(self.pq_ephemeral.public_key.as_bytes());

        self.transcript.extend_from_slice(&msg1);
        stream.write_all(&msg1).await?;

        // Message 2: Receive responder's keys
        let mut msg2 = vec![0u8; 65536];
        let len = stream.read(&mut msg2).await?;
        msg2.truncate(len);
        self.transcript.extend_from_slice(&msg2);

        // Process Noise portion
        let noise_len = /* calculate */;
        let mut payload = vec![0u8; 65536];
        self.noise.read_message(&msg2[..noise_len], &mut payload)?;

        // Decapsulate ML-KEM ciphertext
        let ct_start = noise_len;
        let ct_end = ct_start + ML_KEM_768_CIPHERTEXT_SIZE;
        let pq_ct = ml_kem::Ciphertext::<MlKem768>::from_bytes(
            &msg2[ct_start..ct_end]
        )?;
        let pq_ss = self.pq_ephemeral.secret_key.decapsulate(&pq_ct)?;
        self.pq_secrets.push(*pq_ss.as_ref());

        // ... continue with message 3 ...

        self.derive_session_secrets()
    }

    fn derive_session_secrets(&self) -> Result<SessionSecrets> {
        // Get Noise handshake hash
        let noise_hash = self.noise.get_handshake_hash();

        // Combine all secrets
        let mut combined_input = Vec::new();
        combined_input.extend_from_slice(noise_hash);
        for pq_ss in &self.pq_secrets {
            combined_input.extend_from_slice(pq_ss);
        }
        combined_input.extend_from_slice(&blake3::hash(&self.transcript).as_bytes());

        // Derive session secrets
        let combined = blake3::keyed_hash(
            b"wraith-v2-handshake-combined",
            &combined_input,
        );

        derive_session_keys(combined.as_bytes(), &self.transcript_hash())
    }
}
```

---

## Key Management

### Key Hierarchy

```
Key Hierarchy:
══════════════

Identity Layer (Long-term)
├── Ed25519 Signing Key
│   └── Used for: Identity binding, migration proofs
├── ML-DSA-65 Signing Key (optional)
│   └── Used for: Post-quantum identity (optional)
└── X25519 Static Key
    └── Used for: Noise_XX static DH

Session Layer (Per-connection)
├── X25519 Ephemeral Key
│   └── Used for: Noise_XX ephemeral DH
├── ML-KEM-768 Ephemeral Key
│   └── Used for: Post-quantum key encapsulation
├── Combined Session Secret
│   └── Derived from: All DH and KEM outputs
└── Traffic Secrets
    ├── Client-to-Server Key
    ├── Server-to-Client Key
    ├── Format Key
    └── Initial Chain Key

Packet Layer (Per-packet)
├── Chain Key (ratcheted)
│   └── Derives: Message key, next chain key
└── Message Key (per-packet)
    └── Derives: AEAD key, nonce
```

### Key Storage

```rust
/// Secure key storage interface
pub trait KeyStore: Send + Sync {
    /// Store key material
    fn store(&self, key_id: &KeyId, material: &SecretBytes) -> Result<()>;

    /// Retrieve key material
    fn retrieve(&self, key_id: &KeyId) -> Result<SecretBytes>;

    /// Delete key material
    fn delete(&self, key_id: &KeyId) -> Result<()>;

    /// Rotate key (atomic replace)
    fn rotate(&self, key_id: &KeyId, new_material: &SecretBytes) -> Result<()>;
}

/// Key identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyId {
    pub key_type: KeyType,
    pub identity: [u8; 32],
    pub generation: u64,
}

/// Types of keys stored
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyType {
    IdentityClassical,
    IdentityPostQuantum,
    SessionSecret,
    ChainKey,
    FormatKey,
}

/// Zero-on-drop secret bytes
pub struct SecretBytes(Vec<u8>);

impl Drop for SecretBytes {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
```

---

## Security Analysis

### Threat Model Coverage

| Threat | v1 Mitigation | v2 Mitigation |
|--------|---------------|---------------|
| Passive eavesdropper | XChaCha20 encryption | Same + polymorphic format |
| Active MITM | Noise_XX authentication | Same + hybrid binding |
| Quantum adversary (future) | None | ML-KEM-768 + ML-DSA-65 |
| Key compromise | Per-minute ratchet | Per-packet ratchet |
| Traffic analysis | Fixed padding | Continuous + polymorphic |
| Replay attacks | Sequence numbers | Same + sliding window |

### Cryptographic Assumptions

```
Security relies on hardness of:
════════════════════════════════

Classical:
1. Discrete Logarithm Problem (X25519)
   - Best attack: Pollard's rho, O(2^128) operations

2. ECDLP (Ed25519)
   - Best attack: Pollard's rho, O(2^128) operations

Post-Quantum:
3. Module Learning With Errors (ML-KEM-768)
   - Best known attack: ~2^146 operations (classical)
   - Quantum: ~2^100+ (uncertain)

4. Module SIS (ML-DSA-65)
   - Best known attack: ~2^150 operations (classical)

Hash Function:
5. BLAKE3 Security
   - Collision resistance: 128-bit
   - Preimage resistance: 256-bit
   - Second preimage: 256-bit
```

### Formal Verification Goals

```rust
// Properties to verify (in F* or similar)
//
// 1. Key Indistinguishability
//    combined_secret is indistinguishable from random
//    if EITHER X25519 OR ML-KEM is secure
//
// 2. Forward Secrecy
//    Compromise of chain_key[n] reveals only message_key[n]
//    All chain_key[0..n-1] are deleted
//
// 3. Authentication
//    Successful handshake implies peer knows private key
//    No MITM can complete handshake without detection
//
// 4. Replay Protection
//    Each packet_number used at most once
//    Sliding window rejects old packets
```

---

## Migration Guide

### Crypto Migration Steps

1. **Update Dependencies**

```toml
[dependencies]
# Remove old crypto crates
# x25519-dalek = "1.0"  # Remove

# Add hybrid crypto
wraith-crypto = { version = "2.0", features = ["hybrid-pq"] }
ml-kem = "0.1"
ml-dsa = "0.1"
```

2. **Update Key Generation**

```rust
// Before (v1)
let keypair = x25519_dalek::StaticSecret::random_from_rng(&mut rng);

// After (v2)
let (secret, public) = HybridSecretKey::generate();
```

3. **Update Handshake**

```rust
// Before (v1)
let handshake = NoiseXX::new(keypair);
let session = handshake.complete(stream)?;

// After (v2)
let handshake = HybridHandshake::new_initiator(&secret_key, peer_pq_key)?;
let session = handshake.execute(stream).await?;
```

4. **Update Key Derivation**

```rust
// Before (v1)
let key = hkdf_sha256(secret, b"wraith traffic key", 32);

// After (v2)
let mut key = [0u8; 32];
HkdfBlake3::derive(
    &[],
    secret,
    labels::TRAFFIC_KEY_CLIENT_TO_SERVER,
    &mut key,
);
```

### Compatibility Considerations

- v1 peers cannot perform hybrid handshake
- v2 in compatibility mode falls back to classical-only
- Identity migration requires binding proof

---

## Related Documents

- [Specification](01-WRAITH-Protocol-v2-Specification.md) - Complete protocol spec
- [Security Analysis](06-WRAITH-Protocol-v2-Security-Analysis.md) - Detailed security analysis
- [Migration Guide](09-WRAITH-Protocol-v2-Migration-Guide.md) - Migration instructions
- [Wire Format Changes](11-WRAITH-Protocol-v2-Wire-Format-Changes.md) - Wire format details

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial cryptographic upgrades document |
