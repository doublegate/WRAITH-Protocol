# Phase 1: Crypto Foundation

**Parent:** [v2 Migration Master Plan](../v2-migration-master.md)
**Version:** 1.0.0
**Status:** Planning (Post-v2.3.0)
**Story Points:** 95-120 SP
**Duration:** 2-3 weeks

---

## Executive Summary

Phase 1 establishes the cryptographic foundation for WRAITH Protocol v2, introducing hybrid post-quantum key exchange, enhanced key derivation, and per-packet forward secrecy. This phase is the critical foundation for all subsequent migration work.

### Objectives

1. Implement hybrid X25519 + ML-KEM-768 key exchange
2. Replace HKDF-SHA256 with HKDF-BLAKE3
3. Implement per-packet forward secrecy ratchet
4. Add optional ML-DSA-65 signature support
5. Achieve 95% test coverage on all crypto code

---

## Sprint Breakdown

### Sprint 1.1: ML-KEM-768 Integration (21-26 SP)

**Goal:** Integrate ML-KEM-768 post-quantum KEM.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 1.1.1 | Add `ml-kem` crate dependency | 1 | Critical | - |
| 1.1.2 | Create `MlKem768` wrapper struct | 3 | Critical | - |
| 1.1.3 | Implement `MlKem768::generate()` | 3 | Critical | - |
| 1.1.4 | Implement `MlKem768::encapsulate()` | 5 | Critical | - |
| 1.1.5 | Implement `MlKem768::decapsulate()` | 5 | Critical | - |
| 1.1.6 | Add serialization (public key, ciphertext) | 3 | High | - |
| 1.1.7 | Implement Zeroize on all key types | 2 | Critical | - |
| 1.1.8 | Unit tests (known answer tests) | 5 | Critical | - |
| 1.1.9 | Performance benchmarks | 3 | Medium | - |

**Acceptance Criteria:**
- [ ] ML-KEM-768 generates valid key pairs
- [ ] Encapsulation produces correct shared secret
- [ ] Decapsulation recovers shared secret
- [ ] All key material zeroized on drop
- [ ] KAT vectors pass from NIST test suite
- [ ] Performance: keygen <1ms, encap/decap <500us

**Code Location:** `crates/wraith-crypto/src/pq/mlkem.rs`

---

### Sprint 1.2: Hybrid Key Exchange (26-32 SP)

**Goal:** Combine X25519 and ML-KEM-768 into hybrid KEM.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 1.2.1 | Define `HybridPublicKey` struct | 2 | Critical | - |
| 1.2.2 | Define `HybridSecretKey` struct | 2 | Critical | - |
| 1.2.3 | Define `HybridCiphertext` struct | 2 | Critical | - |
| 1.2.4 | Implement `HybridKeyPair::generate()` | 3 | Critical | - |
| 1.2.5 | Implement hybrid encapsulation | 8 | Critical | - |
| 1.2.6 | Implement hybrid decapsulation | 8 | Critical | - |
| 1.2.7 | Implement secure key combination (BLAKE3) | 5 | Critical | - |
| 1.2.8 | Add classical-only fallback mode | 3 | High | - |
| 1.2.9 | Serialization for all hybrid types | 3 | High | - |
| 1.2.10 | Integration tests (full KEM cycle) | 5 | Critical | - |
| 1.2.11 | Property-based tests (encap/decap inverse) | 5 | High | - |

**Acceptance Criteria:**
- [ ] Hybrid key generation combines both algorithms
- [ ] Encapsulation uses both classical and PQ
- [ ] Shared secret derived from both components
- [ ] Key combination uses domain-separated BLAKE3
- [ ] Classical-only mode works for degraded environments
- [ ] Serialization round-trips correctly

**Key Algorithm:**
```rust
fn combine_shared_secrets(classical: &[u8; 32], post_quantum: &[u8]) -> SharedSecret {
    let mut hasher = blake3::Hasher::new_keyed(b"wraith-hybrid-kem-v2-combine-ss");
    hasher.update(classical);
    hasher.update(post_quantum);
    hasher.update(&(classical.len() as u32).to_le_bytes());
    hasher.update(&(post_quantum.len() as u32).to_le_bytes());
    SharedSecret::from(*hasher.finalize().as_bytes())
}
```

**Code Location:** `crates/wraith-crypto/src/hybrid/`

---

### Sprint 1.3: HKDF-BLAKE3 Migration (13-16 SP)

**Goal:** Replace HKDF-SHA256 with HKDF-BLAKE3.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 1.3.1 | Implement `hkdf_blake3_extract()` | 3 | Critical | - |
| 1.3.2 | Implement `hkdf_blake3_expand()` | 3 | Critical | - |
| 1.3.3 | Create unified `hkdf_blake3()` interface | 2 | Critical | - |
| 1.3.4 | Update all KDF call sites | 5 | Critical | - |
| 1.3.5 | Deprecate `hkdf_sha256()` | 1 | High | - |
| 1.3.6 | KDF test vectors | 3 | Critical | - |
| 1.3.7 | Migration path for existing sessions | 2 | Medium | - |

**Acceptance Criteria:**
- [ ] HKDF-BLAKE3 matches expected test vectors
- [ ] All key derivation uses new KDF
- [ ] Old KDF marked deprecated with warning
- [ ] No performance regression vs SHA256 KDF
- [ ] Label uniqueness verified across codebase

**KDF Labels (v2):**
```
wraith-v2-handshake-init
wraith-v2-handshake-resp
wraith-v2-session-key
wraith-v2-ratchet-chain
wraith-v2-ratchet-message
wraith-v2-stream-key
```

**Code Location:** `crates/wraith-crypto/src/kdf.rs`

---

### Sprint 1.4: Per-Packet Ratchet (21-26 SP)

**Goal:** Implement per-packet forward secrecy ratchet.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 1.4.1 | Define `PacketRatchet` struct | 3 | Critical | - |
| 1.4.2 | Implement chain key derivation | 5 | Critical | - |
| 1.4.3 | Implement message key derivation | 5 | Critical | - |
| 1.4.4 | Implement `advance()` with atomic zeroization | 5 | Critical | - |
| 1.4.5 | Out-of-order key cache (sliding window) | 5 | High | - |
| 1.4.6 | Key cache eviction policy | 3 | High | - |
| 1.4.7 | Zeroizing LRU cache implementation | 3 | Critical | - |
| 1.4.8 | Ratchet state serialization (for migration) | 2 | Medium | - |
| 1.4.9 | Unit tests (forward secrecy verification) | 3 | Critical | - |
| 1.4.10 | Property tests (advance monotonicity) | 3 | High | - |

**Acceptance Criteria:**
- [ ] Each packet gets unique key
- [ ] Previous keys cannot be derived from current
- [ ] Out-of-order packets within window work
- [ ] Old keys properly evicted and zeroized
- [ ] Chain state zeroized atomically during advance
- [ ] Performance: <1us per ratchet advance

**Ratchet Algorithm:**
```rust
impl PacketRatchet {
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
```

**Code Location:** `crates/wraith-crypto/src/ratchet/packet.rs`

---

### Sprint 1.5: ML-DSA-65 Signatures (8-12 SP)

**Goal:** Add optional post-quantum signature support.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 1.5.1 | Add `ml-dsa` crate dependency | 1 | High | - |
| 1.5.2 | Create `MlDsa65` wrapper struct | 2 | High | - |
| 1.5.3 | Implement `MlDsa65::generate()` | 2 | High | - |
| 1.5.4 | Implement `MlDsa65::sign()` | 2 | High | - |
| 1.5.5 | Implement `MlDsa65::verify()` | 2 | High | - |
| 1.5.6 | Hybrid signature (Ed25519 + ML-DSA-65) | 3 | Medium | - |
| 1.5.7 | Unit tests (signature verification) | 2 | High | - |

**Acceptance Criteria:**
- [ ] ML-DSA-65 generates valid key pairs
- [ ] Signatures verify correctly
- [ ] Invalid signatures rejected
- [ ] Optional feature flag for PQ signatures
- [ ] Hybrid binding proof for identity migration

**Code Location:** `crates/wraith-crypto/src/pq/mldsa.rs`

---

### Sprint 1.6: Integration & Documentation (6-8 SP)

**Goal:** Integrate all crypto components and document.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 1.6.1 | Create `CryptoContext` v2 facade | 3 | Critical | - |
| 1.6.2 | Integration tests (full handshake) | 3 | Critical | - |
| 1.6.3 | API documentation (rustdoc) | 2 | High | - |
| 1.6.4 | Security considerations doc update | 1 | High | - |
| 1.6.5 | Benchmark suite finalization | 1 | Medium | - |

**Acceptance Criteria:**
- [ ] `CryptoContext` provides unified v2 API
- [ ] Full handshake with hybrid crypto works
- [ ] All public APIs documented
- [ ] 95% test coverage achieved
- [ ] Benchmarks show target performance

**Code Location:** `crates/wraith-crypto/src/context.rs`

---

## Technical Specifications

### Hybrid KEM Parameters

| Parameter | Value |
|-----------|-------|
| Classical Algorithm | X25519 |
| Classical Public Key | 32 bytes |
| Classical Shared Secret | 32 bytes |
| PQ Algorithm | ML-KEM-768 |
| PQ Public Key | 1,184 bytes |
| PQ Ciphertext | 1,088 bytes |
| PQ Shared Secret | 32 bytes |
| Combined Shared Secret | 32 bytes |
| Total Public Key Size | 1,216 bytes |
| Total Ciphertext Size | 1,120 bytes |

### Per-Packet Ratchet Parameters

| Parameter | Value |
|-----------|-------|
| Chain Key Size | 32 bytes |
| Message Key Size | 32 bytes |
| Hash Algorithm | BLAKE3 |
| Ratchet Advance | Per-packet |
| Out-of-Order Window | 1024 packets |
| Key Cache TTL | 60 seconds |

### Key Derivation Labels

| Context | Label |
|---------|-------|
| Hybrid Combination | `wraith-hybrid-kem-v2-combine-ss` |
| Handshake Init | `wraith-v2-handshake-init` |
| Handshake Response | `wraith-v2-handshake-resp` |
| Session Key | `wraith-v2-session-key` |
| Ratchet Chain | `wraith-v2-ratchet-chain-next` |
| Ratchet Message | `wraith-v2-ratchet-message-key` |
| Stream Key | `wraith-v2-stream-key` |

---

## Testing Requirements

### Test Categories

| Category | Target Coverage | Method |
|----------|-----------------|--------|
| Unit Tests | 95% | Standard test framework |
| Property Tests | Key invariants | `proptest` crate |
| KAT Vectors | 100% NIST vectors | Known answer tests |
| Fuzzing | Crypto boundaries | `libfuzzer` |
| Integration | Full handshake | Multi-crate tests |

### Security Test Cases

| Test Case | Description |
|-----------|-------------|
| T1.1 | Hybrid KEM produces IND-CCA2 secure secrets |
| T1.2 | Per-packet ratchet provides forward secrecy |
| T1.3 | Key zeroization verified (memory inspection) |
| T1.4 | Constant-time operations verified |
| T1.5 | No timing side-channels in KEM |
| T1.6 | Replay detection works correctly |

### Performance Benchmarks

| Benchmark | Target | Notes |
|-----------|--------|-------|
| hybrid_keygen | <1ms | X25519 + ML-KEM-768 |
| hybrid_encapsulate | <500us | Both algorithms |
| hybrid_decapsulate | <500us | Both algorithms |
| ratchet_advance | <1us | BLAKE3-based |
| hkdf_blake3_32 | <1us | 32-byte output |
| mldsa65_sign | <5ms | 64KB message |
| mldsa65_verify | <2ms | 64KB message |

---

## Dependencies

### External Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| ml-kem | 0.2+ | ML-KEM-768 implementation |
| ml-dsa | 0.1+ | ML-DSA-65 implementation |
| x25519-dalek | 2.0 | X25519 (existing) |
| blake3 | 1.5+ | Hashing, KDF |
| zeroize | 1.7+ | Secure memory clearing |
| proptest | 1.4+ | Property-based testing |

### Internal Dependencies

| Crate | Dependency Type |
|-------|----------------|
| wraith-core | Consumer |
| wraith-transport | Consumer |
| wraith-obfuscation | Consumer |

---

## Risk Assessment

### High Risk

| Risk | Mitigation |
|------|------------|
| ML-KEM implementation bugs | Use audited crate, extensive testing |
| Timing side-channels | Constant-time verification tools |
| Key combination weakness | Formal analysis, domain separation |

### Medium Risk

| Risk | Mitigation |
|------|------------|
| Performance regression | Continuous benchmarking |
| Memory leaks of key material | Valgrind, sanitizers |
| API ergonomics | Early feedback, iterative design |

---

## Deliverables Checklist

### Code Deliverables

- [ ] `crates/wraith-crypto/src/pq/mlkem.rs` - ML-KEM-768 wrapper
- [ ] `crates/wraith-crypto/src/pq/mldsa.rs` - ML-DSA-65 wrapper
- [ ] `crates/wraith-crypto/src/hybrid/keypair.rs` - Hybrid key pair
- [ ] `crates/wraith-crypto/src/hybrid/encapsulate.rs` - Hybrid encapsulation
- [ ] `crates/wraith-crypto/src/kdf.rs` - HKDF-BLAKE3
- [ ] `crates/wraith-crypto/src/ratchet/packet.rs` - Per-packet ratchet
- [ ] `crates/wraith-crypto/src/context.rs` - CryptoContext v2

### Test Deliverables

- [ ] Unit tests for all new modules
- [ ] Property tests for crypto invariants
- [ ] NIST KAT vectors for ML-KEM and ML-DSA
- [ ] Integration tests for full handshake
- [ ] Benchmark suite

### Documentation Deliverables

- [ ] Rustdoc for all public APIs
- [ ] Security considerations update
- [ ] Migration notes for v1 crypto users

---

## Gap Analysis (v2.3.7 Assessment)

### Current Implementation State

| Component | Status | Notes |
|-----------|--------|-------|
| ML-KEM-768 basic wrapper | COMPLETE | `crates/wraith-crypto/src/pq.rs` - keygen, encap, decap, serialization exist |
| X25519 | COMPLETE | `crates/wraith-crypto/src/x25519.rs` |
| HKDF-BLAKE3 | PARTIAL | `crates/wraith-crypto/src/hash.rs` has `hkdf_extract`/`hkdf_expand` using BLAKE3 already |
| Double Ratchet | COMPLETE | `crates/wraith-crypto/src/ratchet.rs` - full Signal-style Double Ratchet |
| Ed25519 | COMPLETE | `crates/wraith-crypto/src/signatures.rs` |
| Elligator2 | COMPLETE | `crates/wraith-crypto/src/elligator.rs` |
| Constant-time ops | COMPLETE | `crates/wraith-crypto/src/constant_time.rs` |
| Zeroize | COMPLETE | Used throughout via `zeroize` crate |

### Gaps Identified

1. **Hybrid KEM combiner** (Sprint 1.2): ML-KEM-768 exists but no hybrid X25519+ML-KEM combination logic. Need `combine_shared_secrets()` with domain-separated BLAKE3 as per doc 12. Estimated ~400 lines new code.

2. **Per-packet symmetric ratchet** (Sprint 1.4): Current ratchet is Signal Double Ratchet (per-message chain keys). v2 requires a *separate* per-packet symmetric ratchet using BLAKE3 chain that advances with every packet. This is conceptually simpler than the existing Double Ratchet but operates at a different layer. Estimated ~300 lines.

3. **ML-DSA-65 signatures** (Sprint 1.5): Not present. Needs `ml-dsa` crate integration. Estimated ~200 lines.

4. **Crypto suite negotiation** (NEW - needs Sprint 1.7): v2 spec defines Suite A (default), Suite B (HW accel), Suite C (max security), Suite D (classical-only). No negotiation mechanism exists. Estimated ~500 lines.

5. **KDF label migration**: Current HKDF labels need updating to v2 labels (`wraith-v2-*`). Low effort but critical for correctness.

6. **Algorithm agility framework**: v2 requires negotiated algorithm selection during handshake. Current code hardcodes XChaCha20-Poly1305 + BLAKE3. Estimated ~600 lines for the `CryptoSuite` abstraction.

### Inaccuracies in Current Plan

- Sprint 1.3 (HKDF-BLAKE3): Listed as "Replace HKDF-SHA256". Current implementation already uses HKDF-BLAKE3, not SHA256. Sprint should be re-scoped to KDF label migration and v2 key derivation schedule changes (directional keys, session ID binding, wire format seed derivation).
- Sprint 1.1 story points may be lower since ML-KEM-768 wrapper already exists. Re-estimate to 8-12 SP (from 21-26) for completing hybrid integration.

### New Sprint Required

#### Sprint 1.7: Crypto Suite Negotiation (13-16 SP)

**Goal:** Implement algorithm agility and suite negotiation per doc 12.

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 1.7.1 | Define `CryptoSuite` enum (A/B/C/D) | 2 | Critical |
| 1.7.2 | Define `CryptoSuiteConfig` with algorithm selections | 3 | Critical |
| 1.7.3 | Implement AES-256-GCM AEAD alternative (Suite B) | 3 | Medium |
| 1.7.4 | Suite negotiation during handshake extensions | 5 | Critical |
| 1.7.5 | Unit tests for suite selection | 3 | Critical |

**Acceptance Criteria:**
- [ ] Suite A/B/C/D selectable at configuration time
- [ ] Negotiation selects strongest common suite
- [ ] Fallback to Suite D (classical-only) if PQ not available

**Code Location:** `crates/wraith-crypto/src/suite.rs`

### Revised Story Point Estimate

| Sprint | Original SP | Revised SP | Notes |
|--------|------------|------------|-------|
| 1.1 ML-KEM | 21-26 | 8-12 | pq.rs already exists |
| 1.2 Hybrid KEM | 26-32 | 26-32 | Unchanged |
| 1.3 HKDF-BLAKE3 | 13-16 | 8-10 | Already BLAKE3, just label/schedule migration |
| 1.4 Per-Packet Ratchet | 21-26 | 21-26 | Unchanged |
| 1.5 ML-DSA-65 | 8-12 | 8-12 | Unchanged |
| 1.6 Integration | 6-8 | 6-8 | Unchanged |
| **1.7 Suite Negotiation** | **NEW** | **13-16** | **New sprint** |
| **Total** | **95-120** | **90-116** | Slightly lower due to existing PQ code |

### Phase Dependencies (Outbound)

- Phase 2 depends on: Session secret for polymorphic format derivation (Sprint 1.6)
- Phase 3 depends on: CryptoSuite for transport-specific crypto (Sprint 1.7)
- Phase 7 (Obfuscation) depends on: Per-packet ratchet for continuous padding seed (Sprint 1.4)

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial Phase 1 sprint plan |
| 1.1.0 | 2026-02-01 | Gap analysis, revised estimates, added Sprint 1.7 (Suite Negotiation) |
