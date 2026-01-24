# Phase 4: Integration Testing

**Parent:** [v2 Migration Master Plan](../v2-migration-master.md)
**Version:** 1.0.0
**Status:** Planning (Post-v2.3.0)
**Story Points:** 85-110 SP
**Duration:** 2-3 weeks
**Dependencies:** Phase 3 (Transport)

---

## Executive Summary

Phase 4 focuses on comprehensive integration testing, performance benchmarking, and security validation. This phase ensures v1 to v2 interoperability, validates all performance targets, and confirms security properties through extensive testing.

### Objectives

1. Validate v1 <-> v2 interoperability
2. Test version and feature negotiation
3. Benchmark performance against v1 baseline
4. Security validation (fuzzing, property testing)
5. Compatibility mode validation (90-day window)

---

## Sprint Breakdown

### Sprint 4.1: Interoperability Testing (21-26 SP)

**Goal:** Validate v1 and v2 protocol interoperability.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 4.1.1 | Set up v1 test server infrastructure | 3 | Critical | - |
| 4.1.2 | Set up v2 test server infrastructure | 3 | Critical | - |
| 4.1.3 | Test v1 client -> v2 server (compat mode) | 5 | Critical | - |
| 4.1.4 | Test v2 client -> v1 server (degraded) | 5 | Critical | - |
| 4.1.5 | Test v2 client -> v2 server (native) | 3 | Critical | - |
| 4.1.6 | Test mixed network (v1 + v2 nodes) | 5 | Critical | - |
| 4.1.7 | Version negotiation correctness | 3 | Critical | - |
| 4.1.8 | Feature negotiation correctness | 3 | Critical | - |
| 4.1.9 | Regression test suite (v1 functionality) | 5 | Critical | - |

**Acceptance Criteria:**
- [ ] v1 clients work with v2 servers in compat mode
- [ ] v2 clients work with v1 servers (degraded features)
- [ ] Version negotiation selects correct protocol
- [ ] Feature negotiation enables appropriate features
- [ ] No regressions in v1 functionality

**Interop Test Matrix:**
```
                          Server
                 ┌─────────┬─────────┬─────────┐
                 │ v1.6    │ v2 Compat│ v2 Strict│
        ┌────────┼─────────┼─────────┼─────────┤
        │ v1.6   │ Pass    │ Pass    │ Fail    │
Client  │ v2 Class│ Pass    │ Pass    │ Pass    │
        │ v2 Hybrid│ Fail    │ Pass    │ Pass    │
        └────────┴─────────┴─────────┴─────────┘
```

**Code Location:** `tests/interop/`

---

### Sprint 4.2: Performance Benchmarking (21-26 SP)

**Goal:** Validate all performance targets against v1 baseline.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 4.2.1 | Create benchmark harness | 3 | Critical | - |
| 4.2.2 | Throughput benchmarks (UDP userspace) | 3 | Critical | - |
| 4.2.3 | Throughput benchmarks (AF_XDP) | 3 | Critical | - |
| 4.2.4 | Latency benchmarks (p50, p99, p999) | 3 | Critical | - |
| 4.2.5 | Handshake latency benchmarks | 3 | Critical | - |
| 4.2.6 | Crypto operation benchmarks | 3 | Critical | - |
| 4.2.7 | Memory usage benchmarks | 2 | High | - |
| 4.2.8 | Scalability benchmarks (concurrent sessions) | 3 | High | - |
| 4.2.9 | Connection migration benchmarks | 2 | High | - |
| 4.2.10 | v1 vs v2 comparison report | 3 | Critical | - |
| 4.2.11 | Performance regression CI gate | 3 | High | - |

**Acceptance Criteria:**
- [ ] Throughput: 500 Mbps (UDP), 40 Gbps (AF_XDP)
- [ ] Latency: <500us (AF_XDP), <3ms (UDP)
- [ ] Handshake: <50ms (LAN), <200ms (WAN)
- [ ] Memory: <100KB per session
- [ ] Migration: <50ms handoff time
- [ ] No regression vs v1 baseline

**Benchmark Targets:**
| Metric | v1 Baseline | v2 Target | Acceptable Range |
|--------|-------------|-----------|------------------|
| UDP Throughput | 300 Mbps | 500 Mbps | 450-550 Mbps |
| AF_XDP Throughput | 10 Gbps | 40 Gbps | 35-45 Gbps |
| Packet Latency (p99) | 2ms | 1ms | 0.8-1.2ms |
| Handshake (LAN) | 30ms | 50ms | 40-60ms |
| Memory/Session | 80KB | 100KB | 90-110KB |
| Hybrid KEM | N/A | <1ms | 0.8-1.2ms |

**Code Location:** `benches/v2_performance/`

---

### Sprint 4.3: Security Validation (26-32 SP)

**Goal:** Comprehensive security testing and validation.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 4.3.1 | Set up fuzzing infrastructure | 3 | Critical | - |
| 4.3.2 | Fuzz wire format parser | 5 | Critical | - |
| 4.3.3 | Fuzz handshake state machine | 5 | Critical | - |
| 4.3.4 | Fuzz crypto boundaries | 5 | Critical | - |
| 4.3.5 | Property-based crypto tests | 5 | Critical | - |
| 4.3.6 | Forward secrecy verification | 3 | Critical | - |
| 4.3.7 | Key zeroization verification | 3 | Critical | - |
| 4.3.8 | Timing side-channel analysis | 3 | Critical | - |
| 4.3.9 | Probing resistance validation | 3 | High | - |
| 4.3.10 | Replay attack testing | 2 | Critical | - |
| 4.3.11 | Downgrade attack testing | 3 | Critical | - |
| 4.3.12 | Security test report | 3 | Critical | - |

**Acceptance Criteria:**
- [ ] No crashes from fuzzing (1M+ iterations)
- [ ] All crypto invariants verified
- [ ] Forward secrecy confirmed (per-packet)
- [ ] No timing side-channels detected
- [ ] Replay attacks prevented
- [ ] Downgrade attacks prevented

**Fuzzing Targets:**
```
fuzz/
├── fuzz_targets/
│   ├── frame_parser.rs        # Wire format parsing
│   ├── header_decode.rs       # Header decoding
│   ├── polymorphic_decode.rs  # Polymorphic format
│   ├── handshake_state.rs     # Handshake state machine
│   ├── crypto_hybrid.rs       # Hybrid KEM
│   ├── ratchet_advance.rs     # Packet ratchet
│   └── version_negotiate.rs   # Version negotiation
└── Cargo.toml
```

**Code Location:** `fuzz/`, `tests/security/`

---

### Sprint 4.4: Compatibility Mode Testing (10-14 SP)

**Goal:** Validate 90-day compatibility window behavior.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 4.4.1 | Compat mode enable/disable tests | 2 | Critical | - |
| 4.4.2 | Compat mode timeout behavior | 3 | Critical | - |
| 4.4.3 | Gradual deprecation simulation | 3 | High | - |
| 4.4.4 | v1 client warning messages | 2 | High | - |
| 4.4.5 | Metrics collection (v1 vs v2 usage) | 3 | High | - |
| 4.4.6 | Compat mode configuration tests | 2 | Medium | - |

**Acceptance Criteria:**
- [ ] Compat mode can be enabled/disabled
- [ ] Timeout correctly enforced
- [ ] v1 clients receive deprecation warnings
- [ ] Usage metrics accurately tracked
- [ ] Clean cutover to v2-only possible

**Code Location:** `tests/compat_mode/`

---

### Sprint 4.5: Migration Path Validation (7-12 SP)

**Goal:** Validate end-to-end migration paths.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 4.5.1 | Identity migration test | 3 | Critical | - |
| 4.5.2 | Session state migration test | 3 | Critical | - |
| 4.5.3 | Key migration test | 3 | Critical | - |
| 4.5.4 | Rolling upgrade scenario | 3 | High | - |
| 4.5.5 | Rollback scenario testing | 2 | High | - |
| 4.5.6 | Migration guide verification | 2 | Medium | - |

**Acceptance Criteria:**
- [ ] Identity migration preserves keys
- [ ] Session migration maintains state
- [ ] Rolling upgrades work without downtime
- [ ] Rollback to v1 possible if needed
- [ ] Migration guide accurate

**Code Location:** `tests/migration/`

---

## Technical Specifications

### Test Pyramid

```
                    ┌─────────────┐
                    │    E2E      │  10%
                    │  (50 tests) │
                    ├─────────────┤
                    │ Integration │  20%
                    │ (200 tests) │
                    ├─────────────┤
                    │   Unit      │  70%
                    │(1200 tests) │
                    └─────────────┘
```

### Coverage Targets

| Component | Target Coverage | Priority |
|-----------|-----------------|----------|
| Crypto | 95% | Critical |
| Protocol | 90% | Critical |
| Wire Format | 90% | Critical |
| Transport | 85% | High |
| CLI | 80% | Medium |
| Overall | 80% | Critical |

### CI Pipeline Requirements

```yaml
v2_integration:
  stages:
    - unit_tests:
        coverage: 80%
        timeout: 10m

    - integration_tests:
        matrix: [linux, macos, windows]
        timeout: 30m

    - security_tests:
        fuzzing: 1M iterations
        timing_analysis: true
        timeout: 60m

    - performance_tests:
        benchmarks: true
        regression_gate: true
        timeout: 30m

    - interop_tests:
        v1_server: true
        v2_server: true
        timeout: 20m
```

---

## Testing Requirements

### Test Categories

| Category | Count | Method |
|----------|-------|--------|
| Unit | ~1200 | Standard framework |
| Integration | ~200 | Multi-crate tests |
| E2E | ~50 | Full stack tests |
| Fuzz | 7 targets | libfuzzer |
| Property | ~100 | proptest |
| Security | ~50 | Specialized tools |

### Test Infrastructure

| Component | Purpose |
|-----------|---------|
| Docker Compose | Multi-node test networks |
| Criterion | Performance benchmarks |
| cargo-fuzz | Fuzz testing |
| proptest | Property-based testing |
| miri | Memory safety checks |

---

## Dependencies

### External Tools

| Tool | Version | Purpose |
|------|---------|---------|
| cargo-fuzz | Latest | Fuzzing |
| criterion | 0.5+ | Benchmarking |
| proptest | 1.4+ | Property tests |
| miri | Latest | Memory safety |

### Phase Dependencies

| Dependency | Type | Notes |
|------------|------|-------|
| Phase 1-3 | Required | All components needed |

---

## Risk Assessment

### High Risk

| Risk | Mitigation |
|------|------------|
| Performance regression | CI gate on benchmarks |
| Security vulnerabilities | Extensive fuzzing |
| Interop failures | Comprehensive matrix |

### Medium Risk

| Risk | Mitigation |
|------|------------|
| Test flakiness | Deterministic seeds |
| Coverage gaps | Coverage enforcement |

---

## Deliverables Checklist

### Test Deliverables

- [ ] `tests/interop/` - Interoperability tests
- [ ] `benches/v2_performance/` - Performance benchmarks
- [ ] `fuzz/` - Fuzz targets
- [ ] `tests/security/` - Security tests
- [ ] `tests/compat_mode/` - Compatibility tests
- [ ] `tests/migration/` - Migration tests

### Report Deliverables

- [ ] Performance comparison report (v1 vs v2)
- [ ] Security validation report
- [ ] Interoperability test matrix
- [ ] Coverage report (80%+ overall)
- [ ] Fuzzing coverage report

### CI Deliverables

- [ ] Performance regression gate
- [ ] Security test automation
- [ ] Interop test automation
- [ ] Coverage enforcement

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial Phase 4 sprint plan |
