# Phase 7: Obfuscation Upgrades

**Parent:** [v2 Migration Master Plan](../v2-migration-master.md)
**Version:** 1.0.0
**Status:** Planning (Post-v2.3.7)
**Story Points:** 110-140 SP
**Duration:** 2-3 weeks
**Dependencies:** Phase 1 (Crypto), Phase 2 (Wire Format)

---

## Executive Summary

Phase 7 upgrades the obfuscation layer from v1 fixed padding classes to v2 continuous distributions, adds probing resistance, entropy normalization, decoy traffic improvements, and integrates the polymorphic wire format from Phase 2 into the obfuscation pipeline.

### Objectives

1. Replace fixed padding classes with continuous distributions (per doc 01, section 6.3)
2. Implement active probing resistance with proof-of-knowledge (per doc 01, section 6.5)
3. Add entropy normalization modes (per doc 01, section 6.6)
4. Enhance decoy traffic with mixing strategies (per doc 01, section 6.7)
5. Integrate timing distribution matching (per doc 01, section 6.4)

---

## Current Implementation State

| Component | Status | Location |
|-----------|--------|----------|
| Fixed padding (5 modes) | COMPLETE | `wraith-obfuscation/src/padding.rs` |
| Timing obfuscation (5 modes) | COMPLETE | `wraith-obfuscation/src/timing.rs` |
| TLS mimicry | COMPLETE | `wraith-obfuscation/src/tls_mimicry.rs` |
| WebSocket mimicry | COMPLETE | `wraith-obfuscation/src/websocket_mimicry.rs` |
| DoH tunnel | COMPLETE | `wraith-obfuscation/src/doh_tunnel.rs` |
| Cover traffic | COMPLETE | `wraith-obfuscation/src/cover.rs` |
| Adaptive profiles | COMPLETE | `wraith-obfuscation/src/adaptive.rs` |
| Continuous padding | NOT STARTED | - |
| Probing resistance | NOT STARTED | - |
| Entropy normalization | NOT STARTED | - |
| Decoy streams | NOT STARTED | - |

---

## Sprint Breakdown

### Sprint 7.1: Continuous Padding Distribution (21-26 SP)

**Goal:** Replace fixed padding classes with continuous distributions.

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 7.1.1 | Define `PaddingDistribution` enum (Uniform, HttpsEmpirical, Gaussian, Adaptive) | 3 | Critical |
| 7.1.2 | Implement Uniform distribution sampling | 2 | Critical |
| 7.1.3 | Implement HttpsEmpirical CDF-based sampling | 5 | Critical |
| 7.1.4 | Implement Gaussian distribution sampling | 3 | High |
| 7.1.5 | Implement Adaptive distribution (learns from traffic) | 5 | Medium |
| 7.1.6 | Integrate with frame builder (`build_into_from_parts`) | 3 | Critical |
| 7.1.7 | Deprecate fixed padding classes | 1 | High |
| 7.1.8 | Unit tests (distribution shape validation) | 3 | Critical |
| 7.1.9 | Statistical tests (chi-squared goodness of fit) | 3 | High |

**Code Location:** `crates/wraith-obfuscation/src/padding_v2.rs`

### Sprint 7.2: Probing Resistance (26-32 SP)

**Goal:** Implement active probing resistance per doc 01, section 6.5.

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 7.2.1 | Define `ProbingResistance` config struct | 2 | Critical |
| 7.2.2 | Implement proof-of-knowledge computation (BLAKE3 HMAC) | 3 | Critical |
| 7.2.3 | Implement proof verification with clock skew tolerance | 3 | Critical |
| 7.2.4 | Implement `ProbeResponse::SilentDrop` | 2 | Critical |
| 7.2.5 | Implement `ProbeResponse::MimicTls` | 5 | High |
| 7.2.6 | Implement `ProbeResponse::MimicHttp` | 3 | High |
| 7.2.7 | Implement `ProbeResponse::ProxyToBackend` | 5 | Medium |
| 7.2.8 | Implement service fronting (`FrontingConfig`) | 5 | Medium |
| 7.2.9 | Integration with handshake (Phase 1 proof packet) | 3 | Critical |
| 7.2.10 | Unit tests | 3 | Critical |

**Code Location:** `crates/wraith-obfuscation/src/probing.rs`

### Sprint 7.3: Entropy Normalization (18-23 SP)

**Goal:** Implement ciphertext entropy reduction per doc 01, section 6.6.

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 7.3.1 | Define `EntropyNormalization` enum | 2 | High |
| 7.3.2 | Implement PredictableInsertion encoding/decoding | 5 | High |
| 7.3.3 | Implement Base64 wrapping | 2 | Medium |
| 7.3.4 | Implement JsonWrapper encoding | 5 | Medium |
| 7.3.5 | Implement HttpChunked encoding | 3 | Medium |
| 7.3.6 | Integration with packet pipeline | 3 | High |
| 7.3.7 | Unit tests | 3 | High |

**Code Location:** `crates/wraith-obfuscation/src/entropy.rs`

### Sprint 7.4: Enhanced Decoy Traffic (18-23 SP)

**Goal:** Improve cover traffic with decoy streams and mixing strategies.

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 7.4.1 | Define `DecoyTrafficConfig` with stream count and bandwidth | 3 | High |
| 7.4.2 | Implement `DecoyContentGenerator` variants (Random, CompressibleRandom, ReplayPattern) | 5 | High |
| 7.4.3 | Implement `MixingStrategy::Replace` | 3 | High |
| 7.4.4 | Implement `MixingStrategy::Additive` | 3 | Medium |
| 7.4.5 | Implement `MixingStrategy::Interleave` | 3 | Medium |
| 7.4.6 | Integration with session packet scheduler | 3 | High |
| 7.4.7 | Unit tests | 3 | High |

**Code Location:** `crates/wraith-obfuscation/src/decoy.rs`

### Sprint 7.5: Timing Distribution Matching (13-16 SP)

**Goal:** Extend timing obfuscation with v2 distribution modes.

| ID | Task | SP | Priority |
|----|------|-----|----------|
| 7.5.1 | Add `TimingMode::HttpsBrowsing` with exponential inter-request time | 3 | High |
| 7.5.2 | Add `TimingMode::VideoStreaming` with segment intervals | 3 | Medium |
| 7.5.3 | Add `TimingMode::CustomHmm` for Hidden Markov Model | 5 | Medium |
| 7.5.4 | Integrate timing credits system | 3 | High |
| 7.5.5 | Unit tests | 2 | High |

**Code Location:** `crates/wraith-obfuscation/src/timing_v2.rs`

---

## Client Impact

- wraith-recon: Depends on wraith-obfuscation directly. Needs updated probing resistance APIs.
- All clients benefit from improved traffic analysis resistance but no direct API changes.

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-02-01 | Initial Phase 7 sprint plan |
