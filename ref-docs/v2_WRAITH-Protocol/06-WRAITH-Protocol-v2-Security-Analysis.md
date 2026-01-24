# WRAITH Protocol v2 Security Analysis

**Document Version:** 2.0.0  
**Status:** Security Reference  
**Classification:** Technical Security Analysis  
**Date:** January 2026  

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Threat Model](#2-threat-model)
3. [Cryptographic Security Analysis](#3-cryptographic-security-analysis)
4. [Protocol Security Properties](#4-protocol-security-properties)
5. [Traffic Analysis Resistance](#5-traffic-analysis-resistance)
6. [Active Attack Resistance](#6-active-attack-resistance)
7. [Implementation Security](#7-implementation-security)
8. [Formal Security Guarantees](#8-formal-security-guarantees)
9. [Attack Surface Analysis](#9-attack-surface-analysis)
10. [Security Recommendations](#10-security-recommendations)
11. [Security Audit Checklist](#11-security-audit-checklist)

---

## 1. Introduction

### 1.1 Purpose

This document provides a comprehensive security analysis of WRAITH Protocol v2, examining cryptographic foundations, security properties, threat resistance, and implementation considerations. It serves as a reference for security auditors, implementers, and operators.

### 1.2 Scope

The analysis covers:

- Cryptographic primitive security and composition
- Protocol-level security guarantees
- Traffic analysis and metadata protection
- Active and passive attack resistance
- Implementation security requirements
- Formal security models and proofs

### 1.3 Security Philosophy

WRAITH Protocol v2 is designed with the following security principles:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        WRAITH v2 Security Principles                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. DEFENSE IN DEPTH                                                       │
│     Multiple independent security layers; compromise of one doesn't        │
│     defeat others. Even if obfuscation fails, encryption protects          │
│     content. Even if encryption weakens, authentication persists.          │
│                                                                             │
│  2. FAIL SECURE                                                            │
│     On any error condition, default to the most restrictive behavior.      │
│     Reject malformed packets. Close sessions on repeated failures.         │
│     Never reveal internal state through error messages.                    │
│                                                                             │
│  3. ZERO TRUST                                                             │
│     Assume hostile network. Authenticate and encrypt everything.           │
│     Verify all peer claims. Never trust unauthenticated data.              │
│                                                                             │
│  4. MINIMAL FINGERPRINT                                                    │
│     Protocol traffic should be indistinguishable from noise or             │
│     legitimate traffic. No static patterns. No predictable timing.         │
│                                                                             │
│  5. CRYPTOGRAPHIC AGILITY                                                  │
│     Support multiple algorithm suites. Enable migration when               │
│     vulnerabilities are discovered. Post-quantum readiness.                │
│                                                                             │
│  6. FORWARD SECRECY                                                        │
│     Compromise of long-term keys must not compromise past sessions.        │
│     Compromise of session keys must not compromise other sessions.         │
│     Minimize exposure window through continuous ratcheting.                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Threat Model

### 2.1 Adversary Classes

WRAITH Protocol v2 considers the following adversary classes:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Adversary Taxonomy                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  CLASS 1: Passive Network Observer                                   │   │
│  │                                                                       │   │
│  │  Capabilities:                                                        │   │
│  │  • Observe all network traffic on a path                             │   │
│  │  • Record encrypted traffic for later analysis                       │   │
│  │  • Perform traffic analysis (timing, sizes, patterns)                │   │
│  │  • Correlate traffic across multiple observation points              │   │
│  │                                                                       │   │
│  │  Examples: ISPs, network administrators, passive wiretaps            │   │
│  │                                                                       │   │
│  │  Protection: ✓ FULL (encryption + obfuscation)                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  CLASS 2: Active Network Attacker                                    │   │
│  │                                                                       │   │
│  │  Capabilities:                                                        │   │
│  │  • All Class 1 capabilities                                          │   │
│  │  • Inject packets into the network                                   │   │
│  │  • Modify packets in transit                                         │   │
│  │  • Drop or delay packets selectively                                 │   │
│  │  • Perform man-in-the-middle attacks                                 │   │
│  │                                                                       │   │
│  │  Examples: Compromised routers, active network attackers             │   │
│  │                                                                       │   │
│  │  Protection: ✓ FULL (authentication + integrity + replay protection) │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  CLASS 3: Active Protocol Prober                                     │   │
│  │                                                                       │   │
│  │  Capabilities:                                                        │   │
│  │  • All Class 2 capabilities                                          │   │
│  │  • Send crafted packets to identify protocol                         │   │
│  │  • Analyze server responses to probing                               │   │
│  │  • Fingerprint protocol implementation                               │   │
│  │                                                                       │   │
│  │  Examples: DPI systems, censorship infrastructure                    │   │
│  │                                                                       │   │
│  │  Protection: ✓ FULL (probing resistance + polymorphism)              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  CLASS 4: Quantum Adversary                                          │   │
│  │                                                                       │   │
│  │  Capabilities:                                                        │   │
│  │  • All Class 1-3 capabilities                                        │   │
│  │  • Access to cryptographically relevant quantum computer             │   │
│  │  • Can run Shor's algorithm (breaks ECDH, RSA)                       │   │
│  │  • Can run Grover's algorithm (halves symmetric key strength)        │   │
│  │                                                                       │   │
│  │  Examples: Nation-state adversary (future)                           │   │
│  │                                                                       │   │
│  │  Protection: ✓ FULL via hybrid cryptography (classical + ML-KEM)     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  CLASS 5: Endpoint Compromise                                        │   │
│  │                                                                       │   │
│  │  Capabilities:                                                        │   │
│  │  • Access to endpoint memory and storage                             │   │
│  │  • Read long-term keys and session state                             │   │
│  │  • Observe plaintext before encryption                               │   │
│  │  • Impersonate the compromised endpoint                              │   │
│  │                                                                       │   │
│  │  Examples: Malware, physical access, supply chain attacks            │   │
│  │                                                                       │   │
│  │  Protection: ✗ OUT OF SCOPE (mitigated, not prevented)               │   │
│  │  Mitigations: Memory protection, secure key storage, attestation     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  CLASS 6: Global Passive Adversary                                   │   │
│  │                                                                       │   │
│  │  Capabilities:                                                        │   │
│  │  • Observe ALL network traffic globally                              │   │
│  │  • Correlate timing across all paths                                 │   │
│  │  • Unlimited storage and computation                                 │   │
│  │                                                                       │   │
│  │  Examples: Theoretical/nation-state with global reach                │   │
│  │                                                                       │   │
│  │  Protection: ✗ OUT OF SCOPE (traffic correlation possible)           │   │
│  │  Note: No point-to-point protocol can resist global correlation      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Threat Summary Matrix

| Threat | Class | Protected | Mechanism |
|--------|-------|-----------|-----------|
| Eavesdropping | 1 | ✓ | XChaCha20-Poly1305 AEAD |
| Traffic recording | 1 | ✓ | Hybrid PQ encryption |
| Traffic analysis | 1 | ✓ | Padding, timing, cover traffic |
| Packet injection | 2 | ✓ | AEAD authentication |
| Packet modification | 2 | ✓ | Poly1305 integrity |
| Replay attacks | 2 | ✓ | Nonce + sliding window |
| MITM attacks | 2 | ✓ | Noise_XX mutual auth |
| Protocol fingerprinting | 3 | ✓ | Polymorphic wire format |
| Active probing | 3 | ✓ | Proof-of-knowledge |
| Shor's algorithm | 4 | ✓ | ML-KEM-768 hybrid |
| Grover's algorithm | 4 | ✓ | 256-bit symmetric keys |
| Endpoint malware | 5 | ✗ | Out of scope |
| Global correlation | 6 | ✗ | Out of scope |

### 2.3 Security Assumptions

The protocol's security relies on these assumptions:

1. **Cryptographic Hardness**
   - Decisional Diffie-Hellman (DDH) is hard in Curve25519
   - Module Learning With Errors (MLWE) is hard for ML-KEM parameters
   - XChaCha20 is a secure pseudorandom function
   - Poly1305 provides 128-bit authentication security
   - BLAKE3 is collision-resistant and a secure PRF

2. **Implementation Correctness**
   - Cryptographic primitives are implemented correctly
   - Random number generation is cryptographically secure
   - No timing side-channels in sensitive operations
   - Memory is properly cleared after use

3. **Endpoint Integrity**
   - Endpoints are not compromised at time of key generation
   - Long-term keys are stored securely
   - Peers' public keys are obtained through trusted channels

---

## 3. Cryptographic Security Analysis

### 3.1 Primitive Security Levels

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Cryptographic Primitive Security                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Primitive              Classical    Post-Quantum   Total         Status    │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  KEY EXCHANGE                                                               │
│  X25519                 128-bit      0-bit          128-bit (PQ: broken)   │
│  ML-KEM-768             192-bit*     128-bit        128-bit                │
│  Hybrid (Combined)      128-bit      128-bit        128-bit MINIMUM        │
│                                                                             │
│  * Classical security of ML-KEM is higher, but we claim minimum           │
│                                                                             │
│  ENCRYPTION                                                                 │
│  XChaCha20-Poly1305     256-bit      128-bit**      128-bit MINIMUM        │
│  AES-256-GCM            256-bit      128-bit**      128-bit MINIMUM        │
│                                                                             │
│  ** After Grover's algorithm halves effective key length                  │
│                                                                             │
│  HASHING                                                                    │
│  BLAKE3                 256-bit      128-bit**      128-bit MINIMUM        │
│                                                                             │
│  SIGNATURES                                                                 │
│  Ed25519                128-bit      0-bit          128-bit (PQ: broken)   │
│                                                                             │
│  Note: Signatures not protected against quantum; long-term identity        │
│  keys should transition to PQ signatures when standardized.                │
│                                                                             │
│  OVERALL PROTOCOL SECURITY                                                 │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Classical: 128-bit (limited by X25519, Ed25519)                           │
│  Post-Quantum: 128-bit (limited by ML-KEM-768, Grover on symmetric)        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Hybrid Key Exchange Security

The hybrid key exchange combines X25519 and ML-KEM-768:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Hybrid KEM Security Analysis                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  COMBINATION METHOD                                                        │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  combined_ss = BLAKE3(                                                     │
│      domain_sep: "wraith-hybrid-kem-v2",                                   │
│      input: x25519_ss || ml_kem_ss                                         │
│  )                                                                          │
│                                                                             │
│  SECURITY PROPERTIES                                                       │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  1. IND-CCA2 Security (Indistinguishability under Chosen Ciphertext)       │
│                                                                             │
│     The hybrid is IND-CCA2 secure if EITHER component is IND-CCA2 secure:  │
│                                                                             │
│     Pr[break hybrid] ≤ Pr[break X25519] × Pr[break ML-KEM]                │
│                                                                             │
│     This is because:                                                       │
│     - Adversary needs BOTH shared secrets to derive combined key           │
│     - BLAKE3 hides any partial information about inputs                    │
│     - Domain separation prevents related-key attacks                       │
│                                                                             │
│  2. Forward Secrecy                                                        │
│                                                                             │
│     Both X25519 and ML-KEM use ephemeral keys per session.                 │
│     Compromise of long-term keys doesn't compromise past sessions.         │
│                                                                             │
│  3. Binding Property                                                       │
│                                                                             │
│     Session transcript binds both key exchanges:                           │
│     - Noise protocol includes both public keys in handshake hash           │
│     - ML-KEM ciphertext bound to session via extension mechanism           │
│                                                                             │
│  ATTACK SCENARIOS                                                          │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Scenario A: Classical adversary only                                      │
│  • X25519: Secure (CDH assumption)                                         │
│  • ML-KEM: Secure (MLWE assumption, overkill)                              │
│  • Result: SECURE                                                          │
│                                                                             │
│  Scenario B: Quantum adversary only                                        │
│  • X25519: BROKEN (Shor's algorithm)                                       │
│  • ML-KEM: Secure (no known quantum algorithm for MLWE)                    │
│  • Result: SECURE (ML-KEM provides security)                               │
│                                                                             │
│  Scenario C: X25519 implementation flaw                                    │
│  • X25519: BROKEN (implementation bug)                                     │
│  • ML-KEM: Secure (independent implementation)                             │
│  • Result: SECURE (ML-KEM provides backup)                                 │
│                                                                             │
│  Scenario D: ML-KEM cryptanalytic breakthrough                             │
│  • X25519: Secure (unaffected)                                             │
│  • ML-KEM: BROKEN (new algorithm)                                          │
│  • Result: SECURE (X25519 provides backup)                                 │
│                                                                             │
│  Scenario E: Both broken simultaneously                                    │
│  • X25519: BROKEN                                                          │
│  • ML-KEM: BROKEN                                                          │
│  • Result: BROKEN                                                          │
│                                                                             │
│  Probability of Scenario E is product of individual break probabilities,  │
│  making it negligibly small for independent failure modes.                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 Noise Protocol Security

WRAITH v2 uses an extended Noise_XX pattern:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Extended Noise_XX Security Analysis                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  STANDARD NOISE_XX PATTERN                                                 │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  -> e                          Initiator ephemeral public key              │
│  <- e, ee, s, es               Responder ephemeral, DH, static, DH         │
│  -> s, se                      Initiator static, DH                        │
│                                                                             │
│  WRAITH v2 EXTENDED PATTERN                                                │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  -> proof, e, [pq_pk]          + Probing resistance + optional PQ pubkey   │
│  <- e, ee, [pq_ct], s, es      + optional PQ ciphertext                    │
│  -> s, se, extensions          + extension negotiation                     │
│                                                                             │
│  SECURITY PROPERTIES (per Noise specification)                             │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  After Message 1 (-> e):                                                   │
│  • No confidentiality (ephemeral public key visible)                       │
│  • No authentication                                                       │
│                                                                             │
│  After Message 2 (<- e, ee, s, es):                                        │
│  • Forward secrecy: YES (ee DH with ephemeral keys)                        │
│  • Initiator authentication: NO                                            │
│  • Responder authentication: YES (es DH proves responder identity)         │
│                                                                             │
│  After Message 3 (-> s, se):                                               │
│  • Forward secrecy: YES                                                    │
│  • Mutual authentication: YES (se DH proves initiator identity)            │
│  • Key confirmation: YES (implicit in successful decryption)               │
│                                                                             │
│  HANDSHAKE HASH BINDING                                                    │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  h = BLAKE3("Noise_XX_wraith2_X25519_ChaChaPoly_BLAKE3")                   │
│  h = BLAKE3(h || prologue)        // Bind protocol version                 │
│  h = BLAKE3(h || e_init)          // Bind initiator ephemeral              │
│  h = BLAKE3(h || e_resp)          // Bind responder ephemeral              │
│  h = BLAKE3(h || ENCRYPT(s_resp)) // Bind encrypted responder static       │
│  h = BLAKE3(h || ENCRYPT(s_init)) // Bind encrypted initiator static       │
│                                                                             │
│  Final 'h' is the session transcript hash, binding all handshake data.     │
│  Any modification to any message is detected.                              │
│                                                                             │
│  IDENTITY HIDING                                                           │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Initiator identity: Hidden (encrypted under responder's ephemeral)        │
│  Responder identity: Hidden (encrypted under shared ee secret)             │
│                                                                             │
│  Active attacker cannot learn either identity without:                     │
│  • Knowing responder's static key (to decrypt initiator identity)          │
│  • Knowing initiator's ephemeral key (to decrypt responder identity)       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.4 AEAD Security

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    XChaCha20-Poly1305 Security Analysis                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  CONSTRUCTION                                                              │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  XChaCha20-Poly1305 = HChaCha20 (key derivation) + ChaCha20-Poly1305      │
│                                                                             │
│  subkey = HChaCha20(key, nonce[0:16])                                      │
│  ciphertext, tag = ChaCha20-Poly1305(subkey, nonce[16:24], plaintext, aad) │
│                                                                             │
│  NONCE SPACE                                                               │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  XChaCha20: 192-bit nonce (24 bytes)                                       │
│  - 2^192 possible nonces per key                                           │
│  - Safe for random nonce generation                                        │
│  - Birthday bound: 2^96 messages before collision likely                   │
│                                                                             │
│  WRAITH v2 uses:                                                           │
│  - 64-bit counter nonce (per direction)                                    │
│  - 128 bits of nonce derived from session secret                           │
│  - Guarantees no nonce reuse within session                                │
│                                                                             │
│  SECURITY GUARANTEES                                                       │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  1. Confidentiality (IND-CPA)                                              │
│     Ciphertexts are indistinguishable from random                          │
│     Security: 256-bit (classical), 128-bit (quantum/Grover)                │
│                                                                             │
│  2. Integrity (INT-CTXT)                                                   │
│     Attacker cannot forge valid ciphertext                                 │
│     Security: 128-bit (Poly1305 tag length)                                │
│                                                                             │
│  3. Authenticity                                                           │
│     Only key holder can produce valid tag                                  │
│     Security: 128-bit                                                      │
│                                                                             │
│  NONCE REUSE RESISTANCE                                                    │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  If nonce reused (catastrophic implementation failure):                    │
│  • XOR of plaintexts revealed                                              │
│  • Poly1305 key reuse enables forgery                                      │
│                                                                             │
│  WRAITH v2 mitigations:                                                    │
│  • Counter-based nonces (not random) - no birthday problem                 │
│  • Separate nonce spaces for each direction                                │
│  • Key ratcheting limits exposure window                                   │
│  • Implementation MUST reject duplicate nonces                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.5 Key Derivation Security

```rust
/// Key derivation security analysis
/// 
/// WRAITH v2 uses BLAKE3-based HKDF for key derivation:
/// 
/// PRK = BLAKE3(
///     domain: "wraith-v2-extract",
///     input: salt || IKM
/// )
/// 
/// OKM = BLAKE3(
///     domain: "wraith-v2-expand",
///     input: PRK || info || counter
/// )
/// 
/// Security properties:
/// 1. PRF security: BLAKE3 is a secure PRF
/// 2. Extract randomness: PRK is uniform even if IKM has structure
/// 3. Independence: Different 'info' values produce independent keys
/// 4. Collision resistance: Cannot find two inputs with same output

/// Derived keys and their purposes
pub mod derived_keys {
    /// Session keys (derived once per session)
    pub const INITIATOR_SEND: &str = "i2r-data";
    pub const RESPONDER_SEND: &str = "r2i-data";
    
    /// Obfuscation seeds (for traffic shaping)
    pub const WIRE_FORMAT_SEED: &str = "wire-format";
    pub const PADDING_SEED: &str = "padding";
    pub const TIMING_SEED: &str = "timing";
    
    /// Symmetric ratchet keys
    pub const CHAIN_KEY: &str = "chain";
    pub const MESSAGE_KEY: &str = "message";
    
    /// Connection ID derivation
    pub const CID_KEY: &str = "connection-id";
    
    /// Stateless reset token
    pub const RESET_TOKEN: &str = "reset-token";
}
```

---

## 4. Protocol Security Properties

### 4.1 Security Property Summary

| Property | Guarantee | Mechanism |
|----------|-----------|-----------|
| Confidentiality | Messages readable only by intended recipient | XChaCha20-Poly1305 AEAD |
| Integrity | Modifications detected | Poly1305 authentication tag |
| Authentication | Peer identities verified | Noise_XX static key exchange |
| Forward Secrecy | Past sessions secure after key compromise | Ephemeral DH + ratcheting |
| Post-Compromise Security | Future sessions secure after recovery | DH ratchet |
| Replay Protection | Replayed packets rejected | Nonce + sliding window |
| Key Confirmation | Both parties have same key | Implicit in AEAD success |
| Identity Hiding | Identities not visible to passive observer | Encrypted static keys |

### 4.2 Forward Secrecy Analysis

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Forward Secrecy Mechanism                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  LAYER 1: Session-Level Forward Secrecy                                    │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Each session uses fresh ephemeral keys:                                   │
│                                                                             │
│  Session 1:  e₁ ←──ee──→ e₁'    Independent of Session 2                  │
│  Session 2:  e₂ ←──ee──→ e₂'    Independent of Session 1                  │
│                                                                             │
│  Compromise of static key (s) doesn't reveal ephemeral keys (e).           │
│  Past session keys cannot be derived from static keys alone.               │
│                                                                             │
│  LAYER 2: Symmetric Ratchet (Per-Packet)                                   │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  chain_key[n+1] = BLAKE3(chain_key[n] || "chain")                         │
│  msg_key[n] = BLAKE3(chain_key[n] || "message")                           │
│                                                                             │
│  • Each packet encrypted with unique key                                   │
│  • Chain key immediately overwritten                                       │
│  • Cannot derive msg_key[n-1] from msg_key[n]                             │
│                                                                             │
│  LAYER 3: DH Ratchet (Periodic)                                            │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Every 120 seconds OR 1M packets:                                          │
│                                                                             │
│  1. Generate new ephemeral: e_new                                          │
│  2. Perform DH: dh_out = X25519(e_new, peer_e)                            │
│  3. Optionally: pq_ss = ML-KEM.Decapsulate(pq_ct)                         │
│  4. Derive new chain key: chain_key = KDF(dh_out || pq_ss)                │
│  5. Delete old ephemeral private key                                       │
│                                                                             │
│  This provides:                                                            │
│  • Post-compromise security (new keys after recovery)                      │
│  • Bounded exposure (max 120s of traffic with compromised key)            │
│  • Quantum resistance (optional PQ component)                              │
│                                                                             │
│  EXPOSURE WINDOW ANALYSIS                                                  │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  If attacker compromises current chain_key at time T:                      │
│                                                                             │
│  │← Cannot decrypt (ratcheted) │ Can decrypt │← Cannot decrypt (ratcheted)│
│  │                             │             │                             │
│  ──────────────────────────────T─────────────T+120s─────────────────────── │
│                                │  Window    │                              │
│                                │  ≤ 120s    │                              │
│                                                                             │
│  Practical exposure: Single packet (symmetric ratchet)                     │
│  Maximum exposure: 120 seconds (DH ratchet interval)                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.3 Replay Protection

```rust
/// Replay protection using sliding window algorithm
/// 
/// Each session maintains a replay window for received nonces.
/// Window size: 1024 packets (configurable)
/// 
/// Security properties:
/// - Replayed packets rejected with high probability
/// - Out-of-order delivery supported within window
/// - Memory bounded (O(window_size) bits)

pub struct ReplayWindow {
    /// Highest received nonce
    highest: u64,
    
    /// Bitmap for window below highest
    /// Bit i set if (highest - i) has been received
    bitmap: u128,  // Covers 128 nonces below highest
    
    /// Extended bitmap for larger windows
    extended: BitVec,
}

impl ReplayWindow {
    /// Check if nonce is valid (not replayed, not too old)
    pub fn check(&mut self, nonce: u64) -> Result<(), ReplayError> {
        // Future nonce: always valid (will update window)
        if nonce > self.highest {
            return Ok(());
        }
        
        // Too old: outside window
        let delta = self.highest - nonce;
        if delta >= WINDOW_SIZE {
            return Err(ReplayError::TooOld);
        }
        
        // Within window: check bitmap
        if self.is_set(delta) {
            return Err(ReplayError::Duplicate);
        }
        
        Ok(())
    }
    
    /// Mark nonce as received (call after successful decryption)
    pub fn mark(&mut self, nonce: u64) {
        if nonce > self.highest {
            // Shift window
            let shift = nonce - self.highest;
            self.shift_window(shift);
            self.highest = nonce;
        }
        // Set bit for this nonce
        let delta = self.highest - nonce;
        self.set_bit(delta);
    }
}
```

---

## 5. Traffic Analysis Resistance

### 5.1 Padding Analysis

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Traffic Analysis Resistance                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  THREAT: Statistical Fingerprinting                                        │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Adversary observes packet sizes and attempts to:                          │
│  1. Identify protocol (vs. other traffic)                                  │
│  2. Infer content type (video, file, messaging)                            │
│  3. Correlate sender/receiver                                              │
│                                                                             │
│  v1 VULNERABILITY: Fixed Padding Classes                                   │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  v1 size distribution:                                                     │
│                                                                             │
│  Frequency                                                                  │
│      │                                                                      │
│  100%├─■───────────────────────────────────────────────────────────────     │
│      │ │                                                                    │
│   75%├─┤                                                                    │
│      │ │                                                                    │
│   50%├─┤─────────────■───────■─────────■──────────────────■────────────     │
│      │ │             │       │         │                  │                 │
│   25%├─┤             │       │         │                  │                 │
│      │ │             │       │         │                  │                 │
│    0%└─┴─────────────┴───────┴─────────┴──────────────────┴─────────────    │
│        64           256     512      1024              1472     8960        │
│                                                                             │
│  Distinctive peaks at fixed sizes are trivially fingerprinted.             │
│                                                                             │
│  v2 MITIGATION: Continuous Distribution                                    │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  v2 size distribution (HTTPS-empirical profile):                           │
│                                                                             │
│  Frequency                                                                  │
│      │                                                                      │
│  100%├──╲                                                                   │
│      │   ╲                                                                  │
│   75%├────╲                                                                 │
│      │     ╲                                                                │
│   50%├──────╲                                                               │
│      │       ╲                                                              │
│   25%├────────╲________                                                     │
│      │                 ╲_____________                                       │
│    0%└────────────────────────────────╲___________________________          │
│        64            500           1000           1472                      │
│                                                                             │
│  Matches observed HTTPS traffic; no distinctive peaks.                     │
│                                                                             │
│  AVAILABLE DISTRIBUTIONS                                                   │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  1. Uniform: Random size within [min, MTU]                                 │
│     - Maximum entropy, highest overhead                                    │
│     - Use when content patterns are extremely sensitive                    │
│                                                                             │
│  2. HTTPS Empirical: Matches measured HTTPS traffic                        │
│     - Blends with legitimate web browsing                                  │
│     - Recommended default for internet transit                             │
│                                                                             │
│  3. Gaussian: Bell curve around mean packet size                           │
│     - Natural-looking distribution                                         │
│     - Good for video streaming profiles                                    │
│                                                                             │
│  4. Custom: User-defined CDF                                               │
│     - Match specific target environment                                    │
│     - Requires traffic measurement                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Timing Obfuscation

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Timing Obfuscation Analysis                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  THREAT: Timing Correlation                                                │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Adversary correlates packet timing to:                                    │
│  1. Match traffic flows (entry/exit correlation)                           │
│  2. Identify interactive vs. bulk transfer                                 │
│  3. Fingerprint user behavior patterns                                     │
│                                                                             │
│  MITIGATION TECHNIQUES                                                     │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  1. Constant Rate Mode                                                     │
│                                                                             │
│     Fixed packet rate regardless of application demand:                    │
│                                                                             │
│     App:    ─┬──────────┬───┬───────────────┬─                            │
│             │          │   │               │                               │
│     Wire: ──┼──┼──┼──┼─┼──┼──┼──┼──┼──┼──┼─┼──┼──┼──                      │
│             │  │  │  │ │  │  │  │  │  │  │ │  │  │                         │
│             ↑  ↑  ↑  ↑ ↑  ↑  ↑  ↑  ↑  ↑  ↑ ↑  ↑  ↑                        │
│             └──┴──┴──┴─┴──┴──┴──┴──┴──┴──┴─┴──┴──┴── Fixed interval       │
│                                                                             │
│     + Maximum timing privacy                                               │
│     - High bandwidth overhead (cover traffic)                              │
│     - Increased latency (buffering)                                        │
│                                                                             │
│  2. Jittered Mode                                                          │
│                                                                             │
│     Add random delay within tolerance:                                     │
│                                                                             │
│     App:    ─┬──────────┬───┬───────────────┬─                            │
│             │          │   │               │                               │
│     Wire: ──┼────┼───┼─┼───┼───┼──┼─────┼──┼────┼──                        │
│             │    │   │ │   │   │  │     │  │    │                          │
│             └──┬─┴─┬─┴─┴─┬─┴─┬─┴──┴──┬──┴──┴──┬─┴── Random jitter         │
│                │   │     │   │       │        │                            │
│                          Delay ∈ [0, max_jitter]                           │
│                                                                             │
│     + Moderate timing privacy                                              │
│     + Lower overhead than constant rate                                    │
│     - Some timing leakage remains                                          │
│                                                                             │
│  3. Burst Shaping                                                          │
│                                                                             │
│     Aggregate packets into fixed-size bursts:                              │
│                                                                             │
│     App:    ─┬┬┬────────┬───┬┬┬──────────┬┬─                              │
│             │││        │   │││          ││                                 │
│     Wire: ──┼┼┼────────┼───┼┼┼──────────┼┼──                               │
│             └┴┴─────────┴───┴┴┴──────────┴┴── Burst boundaries             │
│                                                                             │
│     + Natural for bursty applications                                      │
│     + Moderate privacy improvement                                         │
│     - Burst patterns may be distinctive                                    │
│                                                                             │
│  CONFIGURATION                                                             │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  pub struct TimingConfig {                                                 │
│      mode: TimingMode,          // Constant, Jittered, BurstShaped, None   │
│      target_interval: Duration, // For constant rate                       │
│      max_jitter: Duration,      // For jittered mode                       │
│      burst_threshold: usize,    // For burst shaping                       │
│  }                                                                          │
│                                                                             │
│  Recommended profiles:                                                     │
│  • Stealth:      Constant rate, 100ms interval, full cover traffic        │
│  • Balanced:     Jittered, 0-50ms delay                                   │
│  • Performance:  None (minimal latency)                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.3 Cover Traffic

```rust
/// Cover traffic generation for traffic analysis resistance
/// 
/// Cover traffic fills gaps when application has no data to send,
/// making it impossible to distinguish active from idle connections.
/// 
/// Security: Prevents traffic volume analysis
/// Cost: Bandwidth overhead (typically 10-50% in stealth mode)

pub struct CoverTrafficConfig {
    /// Enable cover traffic
    pub enabled: bool,
    
    /// Target bandwidth for cover traffic (bytes/sec)
    /// Cover traffic fills up to this rate when app traffic is below it
    pub target_rate: u64,
    
    /// Minimum interval between cover packets
    pub min_interval: Duration,
    
    /// Maximum interval between cover packets
    pub max_interval: Duration,
    
    /// Distribution for cover packet sizes
    pub size_distribution: SizeDistribution,
}

impl CoverTraffic {
    /// Generate cover packet if needed
    /// Returns None if application traffic is sufficient
    pub fn maybe_generate(&mut self, current_rate: u64) -> Option<CoverPacket> {
        if !self.config.enabled {
            return None;
        }
        
        // Check if cover traffic is needed
        if current_rate >= self.config.target_rate {
            return None;
        }
        
        // Check timing
        let elapsed = self.last_cover.elapsed();
        if elapsed < self.config.min_interval {
            return None;
        }
        
        // Probabilistic generation based on rate deficit
        let deficit_ratio = 1.0 - (current_rate as f64 / self.config.target_rate as f64);
        if self.rng.gen::<f64>() > deficit_ratio {
            return None;
        }
        
        // Generate cover packet
        let size = self.config.size_distribution.sample(&mut self.rng);
        self.last_cover = Instant::now();
        
        Some(CoverPacket {
            data: self.rng.gen::<[u8; MAX_COVER_SIZE]>()[..size].to_vec(),
            is_cover: true,  // Internal flag, not visible on wire
        })
    }
}
```

---

## 6. Active Attack Resistance

### 6.1 Probing Resistance

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Active Probing Resistance                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  THREAT: Protocol Identification via Probing                               │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Censor sends crafted packets to identify WRAITH servers:                  │
│                                                                             │
│  1. Send random garbage → expect specific error response                   │
│  2. Send partial handshake → expect timeout/retry pattern                  │
│  3. Send malformed packet → expect distinctive rejection                   │
│                                                                             │
│  MITIGATION: Proof-of-Knowledge                                            │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Client must prove knowledge of server's public key before server          │
│  responds at all. Without valid proof, server is silent.                   │
│                                                                             │
│  Protocol:                                                                 │
│                                                                             │
│  1. Client computes proof:                                                 │
│     timestamp = current_unix_time()                                        │
│     random = random_bytes(16)                                              │
│     proof = BLAKE3(                                                        │
│         server_public_key ||                                               │
│         timestamp ||                                                       │
│         random ||                                                          │
│         "wraith-proof-v2"                                                  │
│     )                                                                       │
│                                                                             │
│  2. Client sends: [version || timestamp || random || proof || handshake]   │
│                                                                             │
│  3. Server verifies:                                                       │
│     - Timestamp within ±60 seconds of server time                          │
│     - Proof = BLAKE3(own_public_key || timestamp || random || domain)      │
│     - If invalid: NO RESPONSE (silent drop)                                │
│                                                                             │
│  4. If valid: Server continues normal handshake                            │
│                                                                             │
│  SECURITY ANALYSIS                                                         │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Attacker WITHOUT server public key:                                       │
│  • Cannot compute valid proof                                              │
│  • Probability of random guess: 2^-256                                     │
│  • Server remains completely silent                                        │
│  • Indistinguishable from: no service, firewall drop, unreachable host    │
│                                                                             │
│  Attacker WITH server public key (e.g., from out-of-band):                │
│  • Can compute valid proof                                                 │
│  • Can elicit server response                                              │
│  • But this means server identity already known (enumeration attack)       │
│  • Mitigation: Rotate server keys, use bridging                           │
│                                                                             │
│  Replay attack:                                                            │
│  • Timestamp within ±60 seconds                                            │
│  • Each proof valid only briefly                                           │
│  • Cannot replay old proofs after window expires                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 6.2 Polymorphic Wire Format

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Polymorphic Wire Format                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  THREAT: Wire Format Fingerprinting                                        │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  DPI systems fingerprint protocols by:                                     │
│  • Fixed header positions                                                  │
│  • Magic bytes at known offsets                                            │
│  • Field size patterns                                                     │
│                                                                             │
│  v1 VULNERABILITY: Fixed Format                                            │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  v1 packet (fixed layout):                                                 │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ CID (8B) │ Encrypted Payload (variable) │ Auth Tag (16B)            │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  Always: 8 bytes at start, 16 bytes at end → fingerprintable              │
│                                                                             │
│  v2 MITIGATION: Session-Derived Format                                     │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Wire format parameters derived from session secret:                       │
│                                                                             │
│  format_seed = HKDF(session_secret, "wire-format-v2", 32)                  │
│                                                                             │
│  From format_seed, derive:                                                 │
│  • cid_position: 0-3 (start, after_dummy, before_tag, end)                │
│  • cid_size: 4-8 bytes                                                     │
│  • dummy_size: 0-16 bytes (random-looking filler)                          │
│  • length_field: present or absent                                         │
│  • field_order: permutation of fields                                      │
│                                                                             │
│  Example derived formats:                                                  │
│                                                                             │
│  Session A: [Dummy:8][CID:6][Payload][Tag:16]                              │
│  Session B: [CID:4][Payload][Tag:16][Dummy:4]                              │
│  Session C: [Length:2][Payload][CID:8][Tag:16]                             │
│  Session D: [CID:5][Dummy:11][Payload][Tag:16]                             │
│                                                                             │
│  SECURITY ANALYSIS                                                         │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Format entropy: ~64 bits (sufficient to prevent enumeration)              │
│  Format is: deterministic (both endpoints derive same), but                │
│             unique per session, and                                        │
│             indistinguishable from random to observer                      │
│                                                                             │
│  DPI cannot:                                                               │
│  • Know field positions without session secret                             │
│  • Build fixed-offset signatures                                           │
│  • Distinguish from encrypted noise                                        │
│                                                                             │
│  Combined with Elligator2 (public keys look random) and                    │
│  encrypted payload, entire packet is indistinguishable from random.        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Implementation Security

### 7.1 Side-Channel Mitigations

```rust
/// Side-channel mitigation requirements
/// 
/// Implementations MUST follow these requirements to prevent
/// side-channel attacks that could leak secret information.

pub mod side_channel {
    /// Constant-time comparison for secret data
    /// 
    /// REQUIREMENT: All comparisons of secret data (keys, MACs, etc.)
    /// MUST use constant-time comparison to prevent timing attacks.
    /// 
    /// DO NOT USE: slice comparison (==), iterators with early exit
    /// DO USE: subtle::ConstantTimeEq or equivalent
    pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        use subtle::ConstantTimeEq;
        a.ct_eq(b).into()
    }
    
    /// Memory locking for key material
    /// 
    /// REQUIREMENT: Long-term keys and session keys SHOULD be stored
    /// in memory-locked pages to prevent swapping to disk.
    /// 
    /// On Linux: mlock(2)
    /// On Windows: VirtualLock
    /// Fallback: Accept risk, log warning
    pub fn lock_memory(ptr: *mut u8, len: usize) -> Result<(), Error> {
        #[cfg(unix)]
        unsafe {
            if libc::mlock(ptr as *const _, len) != 0 {
                return Err(Error::MemoryLockFailed);
            }
        }
        Ok(())
    }
    
    /// Zeroization on drop
    /// 
    /// REQUIREMENT: All types containing secret data MUST implement
    /// Zeroize and ZeroizeOnDrop to clear memory when dropped.
    /// 
    /// This prevents secrets from lingering in memory after use.
    use zeroize::{Zeroize, ZeroizeOnDrop};
    
    #[derive(Zeroize, ZeroizeOnDrop)]
    pub struct SecretKey([u8; 32]);
    
    /// Constant-time conditional selection
    /// 
    /// REQUIREMENT: Selection between secret values MUST NOT use
    /// branching that depends on secrets.
    /// 
    /// DO NOT USE: if secret_bit { a } else { b }
    /// DO USE: subtle::ConditionallySelectable
    pub fn ct_select<T: subtle::ConditionallySelectable>(
        a: &T, 
        b: &T, 
        choice: subtle::Choice
    ) -> T {
        T::conditional_select(a, b, choice)
    }
}
```

### 7.2 Error Handling Security

```rust
/// Secure error handling requirements
/// 
/// Error messages MUST NOT leak secret information through:
/// - Detailed error descriptions
/// - Different error codes for different failure modes
/// - Timing differences in error paths

pub enum PublicError {
    /// Generic protocol error (hides specific failure)
    ProtocolError,
    
    /// Connection closed (no details)
    ConnectionClosed,
    
    /// Timeout (same timing for all timeout causes)
    Timeout,
}

/// Internal error with details (never exposed to peer)
#[derive(Debug)]
pub(crate) enum InternalError {
    /// Decryption failed (bad key, corrupted, tampered)
    DecryptionFailed { nonce: u64, expected_tag: [u8; 16] },
    
    /// MAC verification failed
    MacMismatch { received: [u8; 16], computed: [u8; 16] },
    
    /// Replay detected
    ReplayDetected { nonce: u64 },
    
    /// Malformed packet
    MalformedPacket { offset: usize, expected: &'static str },
}

impl From<InternalError> for PublicError {
    fn from(_: InternalError) -> Self {
        // Always return generic error
        // Timing MUST be constant regardless of internal error type
        PublicError::ProtocolError
    }
}

/// Error response timing
/// 
/// REQUIREMENT: Error responses MUST have consistent timing to prevent
/// timing oracles. Use constant-time error paths.
pub fn handle_error(err: InternalError) -> PublicError {
    // Log internally (for debugging, not sent to peer)
    tracing::debug!(?err, "internal error");
    
    // Add random delay to mask processing time differences
    // This prevents timing attacks that distinguish error types
    std::thread::sleep(random_delay(1..10)); // 1-10ms
    
    PublicError::ProtocolError
}
```

### 7.3 Random Number Generation

```rust
/// Cryptographic random number generation requirements
/// 
/// All randomness in WRAITH MUST come from cryptographically secure
/// random number generators (CSPRNGs).

pub mod random {
    use rand::{CryptoRng, RngCore};
    
    /// REQUIREMENT: Use only CSPRNGs for security-critical randomness
    /// 
    /// Acceptable sources:
    /// - getrandom crate (uses OS CSPRNG)
    /// - rand::rngs::OsRng
    /// - ring::rand::SystemRandom
    /// 
    /// NOT acceptable:
    /// - rand::thread_rng() without verification
    /// - Any seedable RNG with predictable seed
    /// - Time-based seeding
    pub fn secure_random_bytes(buf: &mut [u8]) {
        use getrandom::getrandom;
        getrandom(buf).expect("CSPRNG failure is fatal");
    }
    
    /// REQUIREMENT: Verify CSPRNG health on startup
    /// 
    /// Some platforms may have depleted entropy pools or broken RNGs.
    /// Verify basic functionality before using in production.
    pub fn verify_rng_health() -> Result<(), Error> {
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        
        secure_random_bytes(&mut buf1);
        secure_random_bytes(&mut buf2);
        
        // Extremely basic check: outputs should differ
        if buf1 == buf2 {
            return Err(Error::RngFailure);
        }
        
        // Check for obvious patterns (all zeros, all ones)
        if buf1.iter().all(|&b| b == 0) || buf1.iter().all(|&b| b == 0xff) {
            return Err(Error::RngFailure);
        }
        
        Ok(())
    }
}
```

---

## 8. Formal Security Guarantees

### 8.1 Security Definitions

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Formal Security Definitions                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  IND-CPA (Indistinguishability under Chosen Plaintext Attack)              │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Game: Adversary chooses m₀, m₁ of equal length                            │
│        Challenger encrypts m_b for random b ∈ {0,1}                        │
│        Adversary outputs guess b'                                          │
│                                                                             │
│  Advantage = |Pr[b' = b] - 1/2|                                            │
│                                                                             │
│  WRAITH Guarantee: Adv^{IND-CPA} ≤ negl(λ) for 128-bit security λ         │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────   │
│                                                                             │
│  IND-CCA2 (Indistinguishability under Adaptive Chosen Ciphertext Attack)   │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Game: IND-CPA + adversary gets decryption oracle                          │
│        (cannot query challenge ciphertext)                                 │
│                                                                             │
│  WRAITH Guarantee: Adv^{IND-CCA2} ≤ negl(λ) via AEAD security              │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────   │
│                                                                             │
│  INT-CTXT (Ciphertext Integrity)                                           │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Game: Adversary tries to produce valid ciphertext not from oracle         │
│                                                                             │
│  WRAITH Guarantee: Adv^{INT-CTXT} ≤ negl(λ) via Poly1305                   │
│                    (128-bit authentication tag)                            │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────   │
│                                                                             │
│  Forward Secrecy                                                           │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Definition: Compromise of long-term keys does not compromise              │
│              session keys from past sessions.                              │
│                                                                             │
│  WRAITH Guarantee: Session keys derived from ephemeral DH (ee)             │
│                    Ephemeral keys deleted after session establishment      │
│                    Past sessions remain secure after key compromise        │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────   │
│                                                                             │
│  Post-Compromise Security (Healing)                                        │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Definition: After temporary compromise, security is restored              │
│              once attacker loses access.                                   │
│                                                                             │
│  WRAITH Guarantee: DH ratchet introduces fresh randomness                  │
│                    After one ratchet, attacker cannot derive new keys      │
│                    Maximum exposure: one ratchet interval (120s)           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Reduction Proofs (Sketch)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Security Reduction Sketches                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Theorem 1: WRAITH Key Exchange is IND-CCA2 secure                         │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Proof sketch:                                                             │
│                                                                             │
│  Assume adversary A breaks WRAITH key exchange.                            │
│  Construct adversary B that breaks either X25519 or ML-KEM.                │
│                                                                             │
│  B receives challenge (pk*, ct*) from challenger.                          │
│  B embeds pk* in WRAITH handshake as either:                               │
│    - X25519 ephemeral public key, OR                                       │
│    - ML-KEM public key                                                     │
│                                                                             │
│  If A distinguishes WRAITH session key:                                    │
│    - If A used X25519 component: B breaks DDH                              │
│    - If A used ML-KEM component: B breaks MLWE                             │
│                                                                             │
│  Since both are assumed hard:                                              │
│    Adv_WRAITH ≤ Adv_X25519 + Adv_ML-KEM ≤ negl(λ)                         │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────   │
│                                                                             │
│  Theorem 2: WRAITH AEAD is INT-CTXT secure                                 │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Proof sketch:                                                             │
│                                                                             │
│  XChaCha20-Poly1305 is proven INT-CTXT under standard assumptions.         │
│  WRAITH uses XChaCha20-Poly1305 with:                                      │
│    - Unique keys per session (from key exchange)                           │
│    - Unique nonces per message (counter-based)                             │
│                                                                             │
│  Given these conditions:                                                   │
│    Adv^{INT-CTXT}_WRAITH = Adv^{INT-CTXT}_XChaCha20-Poly1305 ≤ negl(λ)    │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────   │
│                                                                             │
│  Theorem 3: WRAITH provides forward secrecy                                │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Proof sketch:                                                             │
│                                                                             │
│  Session key = KDF(ee || es || se || PQ_SS)                                │
│                                                                             │
│  where ee = DH(e_init, e_resp) uses ephemeral keys.                        │
│                                                                             │
│  Even with static keys (s_init, s_resp):                                   │
│    - Cannot compute ee without ephemeral private keys                      │
│    - Ephemeral keys deleted after handshake                                │
│    - Past session keys are unrecoverable                                   │
│                                                                             │
│  Therefore, compromise of static keys does not reveal past sessions.       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 9. Attack Surface Analysis

### 9.1 Attack Surface Enumeration

| Surface | Entry Points | Mitigations |
|---------|--------------|-------------|
| Network | UDP/TCP listeners, all transports | Input validation, rate limiting, probing resistance |
| Parser | Packet parsing, frame decoding | Strict parsing, length checks, fuzzing |
| Crypto | Key exchange, AEAD operations | Validated libraries, constant-time |
| State | Session state, replay windows | Bounded memory, timeout cleanup |
| Config | Configuration files, CLI args | Input validation, secure defaults |
| Memory | Buffer handling, allocations | Bounds checking, pool allocation |

### 9.2 Parser Hardening

```rust
/// Parser hardening requirements
/// 
/// All parsing code MUST follow defensive programming practices
/// to prevent exploitation of malformed input.

pub mod parser {
    /// REQUIREMENT: Validate all lengths before allocation
    /// 
    /// Never allocate based on untrusted length without bounds check.
    pub fn read_length_prefixed(buf: &[u8]) -> Result<&[u8], ParseError> {
        if buf.len() < 2 {
            return Err(ParseError::BufferTooShort);
        }
        
        let len = u16::from_be_bytes([buf[0], buf[1]]) as usize;
        
        // Check against maximum allowed
        if len > MAX_FIELD_SIZE {
            return Err(ParseError::LengthTooLarge);
        }
        
        // Check against available data
        if buf.len() < 2 + len {
            return Err(ParseError::BufferTooShort);
        }
        
        Ok(&buf[2..2+len])
    }
    
    /// REQUIREMENT: Use checked arithmetic
    /// 
    /// All offset calculations MUST use checked operations.
    pub fn advance(offset: &mut usize, amount: usize, limit: usize) -> Result<(), ParseError> {
        let new_offset = offset.checked_add(amount)
            .ok_or(ParseError::IntegerOverflow)?;
        
        if new_offset > limit {
            return Err(ParseError::OffsetOutOfBounds);
        }
        
        *offset = new_offset;
        Ok(())
    }
    
    /// REQUIREMENT: Reject ambiguous input
    /// 
    /// If multiple interpretations are possible, reject.
    pub fn parse_strict<T: StrictParseable>(buf: &[u8]) -> Result<T, ParseError> {
        let (value, consumed) = T::parse(buf)?;
        
        // Reject if trailing data exists
        if consumed != buf.len() {
            return Err(ParseError::TrailingData);
        }
        
        Ok(value)
    }
}
```

---

## 10. Security Recommendations

### 10.1 Deployment Recommendations

| Recommendation | Priority | Rationale |
|----------------|----------|-----------|
| Enable post-quantum | High | "Harvest now, decrypt later" threat |
| Use stealth profile in censored networks | High | Detection leads to blocking |
| Rotate server keys periodically | Medium | Limits enumeration attacks |
| Enable certificate pinning | Medium | Prevents MITM via CA compromise |
| Monitor for anomalies | Medium | Early detection of attacks |
| Use memory-safe language (Rust) | High | Prevents memory corruption |
| Run security audit before production | Critical | Verify implementation correctness |

### 10.2 Operational Security

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Operational Security Checklist                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  KEY MANAGEMENT                                                            │
│  □ Generate keys on secure, isolated system                                │
│  □ Store long-term keys in HSM or secure enclave where possible            │
│  □ Implement key rotation policy (recommended: 90 days)                    │
│  □ Maintain secure backup of keys                                          │
│  □ Document key recovery procedures                                        │
│                                                                             │
│  DEPLOYMENT                                                                │
│  □ Use minimal base image (reduce attack surface)                          │
│  □ Run as non-root user                                                    │
│  □ Enable OS-level security (SELinux, AppArmor)                            │
│  □ Configure firewall (allow only necessary ports)                         │
│  □ Enable automatic security updates                                       │
│                                                                             │
│  MONITORING                                                                │
│  □ Log connection attempts (rate, source IPs)                              │
│  □ Alert on unusual traffic patterns                                       │
│  □ Monitor resource usage (DoS detection)                                  │
│  □ Track failed authentication attempts                                    │
│                                                                             │
│  INCIDENT RESPONSE                                                         │
│  □ Document key compromise procedure                                       │
│  □ Prepare emergency key rotation capability                               │
│  □ Maintain contact list for security incidents                            │
│  □ Test recovery procedures periodically                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 11. Security Audit Checklist

### 11.1 Code Audit Checklist

| Area | Check | Status |
|------|-------|--------|
| **Cryptography** | | |
| | Uses well-known, audited crypto libraries | ☐ |
| | No custom cryptographic implementations | ☐ |
| | Constant-time operations for all secret data | ☐ |
| | Proper key derivation with domain separation | ☐ |
| | Nonces never reused | ☐ |
| **Memory Safety** | | |
| | No buffer overflows | ☐ |
| | Bounds checking on all array access | ☐ |
| | Secrets zeroized after use | ☐ |
| | Memory-locked for keys (where supported) | ☐ |
| **Error Handling** | | |
| | Errors don't leak secret information | ☐ |
| | Consistent timing on error paths | ☐ |
| | All errors logged appropriately | ☐ |
| **Input Validation** | | |
| | All inputs validated before use | ☐ |
| | Length limits enforced | ☐ |
| | Malformed input rejected gracefully | ☐ |
| **Protocol** | | |
| | State machine correctly implemented | ☐ |
| | Replay protection working | ☐ |
| | Forward secrecy verified | ☐ |
| | Key ratcheting tested | ☐ |

### 11.2 Fuzzing Targets

```rust
/// Fuzzing target definitions for security testing
/// 
/// Run with: cargo +nightly fuzz run <target>

// Target 1: Packet parser
fuzz_target!(|data: &[u8]| {
    let _ = Packet::parse(data);
});

// Target 2: Handshake state machine
fuzz_target!(|messages: Vec<Vec<u8>>| {
    let mut state = HandshakeState::new_responder(test_keys());
    for msg in messages {
        let _ = state.process_message(&msg);
    }
});

// Target 3: Frame decoder
fuzz_target!(|data: &[u8]| {
    let session = test_session();
    let _ = session.decrypt_frame(data);
});

// Target 4: Obfuscation layer
fuzz_target!(|data: &[u8]| {
    let config = ObfuscationConfig::default();
    let _ = deobfuscate(data, &config);
});
```

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2026-01 | Initial security analysis |

---

*End of WRAITH Protocol v2 Security Analysis*
