# Phase 5: Client Updates

**Parent:** [v2 Migration Master Plan](../v2-migration-master.md)
**Version:** 1.0.0
**Status:** Planning (Post-v2.3.0)
**Story Points:** 150-180 SP
**Duration:** 3-4 weeks
**Dependencies:** Phase 4 (Integration Testing)

---

## Executive Summary

Phase 5 migrates all WRAITH Protocol client applications to v2, including desktop Tauri apps, mobile clients (Android/iOS), and the CLI. This phase updates the FFI layer and ensures feature parity across all platforms.

### Objectives

1. Update wraith-ffi for v2 API
2. Migrate wraith-cli to v2
3. Migrate desktop clients (Transfer, Chat, Sync)
4. Migrate mobile clients (Android, iOS)
5. Prepare future client stubs (Share, Stream, etc.)

---

## Client Inventory

### Active Clients (Migration Required)

| Client | Platform | Language | FFI | Priority |
|--------|----------|----------|-----|----------|
| wraith-cli | All | Rust | N/A | Critical |
| wraith-transfer | Desktop | Rust/TypeScript | N/A | Critical |
| wraith-chat | Desktop | Rust/TypeScript | N/A | Critical |
| wraith-sync | Desktop | Rust/TypeScript | N/A | High |
| wraith-android | Android | Kotlin | JNI | High |
| wraith-ios | iOS | Swift | UniFFI | High |

### Future Clients (Stub Preparation)

| Client | Platform | Status | Priority |
|--------|----------|--------|----------|
| wraith-share | Desktop + Web | Planned | Medium |
| wraith-stream | Desktop + Web | Planned | Medium |
| wraith-mesh | Embedded | Planned | Low |
| wraith-publish | Desktop + Web | Planned | Low |
| wraith-vault | Desktop + CLI | Planned | Low |

---

## Sprint Breakdown

### Sprint 5.1: FFI Layer Update (21-26 SP)

**Goal:** Update wraith-ffi for v2 API compatibility.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 5.1.1 | Update C header generation | 3 | Critical | - |
| 5.1.2 | Add hybrid key types to FFI | 3 | Critical | - |
| 5.1.3 | Update session creation API | 3 | Critical | - |
| 5.1.4 | Add multi-stream FFI functions | 5 | Critical | - |
| 5.1.5 | Add transport migration FFI | 3 | High | - |
| 5.1.6 | Update crypto context FFI | 3 | Critical | - |
| 5.1.7 | Add v1 compat mode FFI flag | 2 | High | - |
| 5.1.8 | Memory safety audit | 3 | Critical | - |
| 5.1.9 | FFI documentation update | 2 | High | - |
| 5.1.10 | FFI test suite update | 3 | Critical | - |

**Acceptance Criteria:**
- [ ] All v2 types exposed via FFI
- [ ] Hybrid crypto available to FFI clients
- [ ] Session builder pattern via FFI
- [ ] Multi-stream support exposed
- [ ] Memory safety verified

**FFI API Changes:**
```c
// New v2 session creation
wraith_session_t* wraith_session_builder_new(void);
int wraith_session_builder_set_connection_id(wraith_session_t* b, const uint8_t* cid, size_t len);
int wraith_session_builder_set_crypto_context(wraith_session_t* b, wraith_crypto_context_t* ctx);
int wraith_session_builder_set_transport(wraith_session_t* b, wraith_transport_t* t);
int wraith_session_builder_set_v1_compat(wraith_session_t* b, bool enable);
wraith_session_t* wraith_session_builder_build(wraith_session_t* b);

// New hybrid crypto
wraith_hybrid_keypair_t* wraith_hybrid_keypair_generate(void);
int wraith_hybrid_encapsulate(wraith_hybrid_keypair_t* kp, const uint8_t* peer_pub,
                              uint8_t* ss_out, uint8_t* ct_out);
int wraith_hybrid_decapsulate(wraith_hybrid_keypair_t* kp, const uint8_t* ct,
                              uint8_t* ss_out);

// Multi-stream
wraith_stream_t* wraith_stream_open(wraith_session_t* s, int priority);
int wraith_stream_send(wraith_stream_t* s, const uint8_t* data, size_t len);
int wraith_stream_recv(wraith_stream_t* s, uint8_t* buf, size_t len);
int wraith_stream_close(wraith_stream_t* s);
```

**Code Location:** `crates/wraith-ffi/`

---

### Sprint 5.2: CLI Migration (18-23 SP)

**Goal:** Migrate wraith-cli to v2 protocol.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 5.2.1 | Update session creation to builder | 3 | Critical | - |
| 5.2.2 | Add hybrid crypto options | 3 | Critical | - |
| 5.2.3 | Add transport selection flags | 3 | Critical | - |
| 5.2.4 | Add multi-stream support | 5 | High | - |
| 5.2.5 | Add v1 compat mode flag | 2 | High | - |
| 5.2.6 | Update config file format | 2 | High | - |
| 5.2.7 | Add migration subcommand | 3 | Medium | - |
| 5.2.8 | Update help text and docs | 2 | High | - |
| 5.2.9 | CLI regression tests | 3 | Critical | - |

**Acceptance Criteria:**
- [ ] All v2 features accessible via CLI
- [ ] `--crypto hybrid` enables PQ crypto
- [ ] `--transport` selects transport type
- [ ] `--compat` enables v1 compatibility
- [ ] Existing workflows still work

**New CLI Options:**
```
wraith send --file data.zip --peer <address>
          --crypto hybrid|classical       # Default: hybrid
          --transport udp|tcp|ws|quic    # Default: udp
          --compat                        # Enable v1 compat
          --streams 4                     # Parallel streams

wraith migrate identity                   # Migrate v1 identity to v2
wraith config set crypto.default hybrid   # Set defaults
```

**Code Location:** `crates/wraith-cli/`

---

### Sprint 5.3: Desktop Clients Migration (34-42 SP)

**Goal:** Migrate Tauri desktop clients to v2.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 5.3.1 | Update wraith-transfer Rust backend | 8 | Critical | - |
| 5.3.2 | Update wraith-transfer TypeScript | 5 | Critical | - |
| 5.3.3 | Update wraith-chat Rust backend | 8 | Critical | - |
| 5.3.4 | Update wraith-chat TypeScript | 5 | Critical | - |
| 5.3.5 | Update wraith-sync Rust backend | 5 | High | - |
| 5.3.6 | Update wraith-sync TypeScript | 3 | High | - |
| 5.3.7 | Add transport selector UI | 3 | Medium | - |
| 5.3.8 | Add crypto mode indicator | 2 | Medium | - |
| 5.3.9 | Integration tests for all clients | 5 | Critical | - |

**Acceptance Criteria:**
- [ ] All desktop clients use v2 protocol
- [ ] Hybrid crypto enabled by default
- [ ] Transport selection available
- [ ] UI indicates security mode
- [ ] Backward compatible with v1 peers (compat mode)

**Desktop Client Updates:**

**wraith-transfer:**
- Session creation via builder
- Multi-stream for parallel chunk transfers
- Transport migration during transfer
- Progress UI shows crypto mode

**wraith-chat:**
- Double Ratchet + per-packet ratchet integration
- Multi-stream for voice/video/data
- Transport selection for call quality
- Group chat with v2 Sender Keys

**wraith-sync:**
- Background sync with v2 protocol
- Delta sync over multi-stream
- Transport failover during sync

**Code Location:** `clients/wraith-transfer/`, `clients/wraith-chat/`, `clients/wraith-sync/`

---

### Sprint 5.4: Mobile Clients Migration (42-52 SP)

**Goal:** Migrate Android and iOS clients to v2.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 5.4.1 | Update Android JNI bindings | 8 | Critical | - |
| 5.4.2 | Update Android Kotlin layer | 5 | Critical | - |
| 5.4.3 | Update Android Keystore integration | 3 | Critical | - |
| 5.4.4 | Android v2 feature integration | 5 | Critical | - |
| 5.4.5 | Android UI updates | 3 | High | - |
| 5.4.6 | Update iOS UniFFI bindings | 8 | Critical | - |
| 5.4.7 | Update iOS Swift layer | 5 | Critical | - |
| 5.4.8 | Update iOS Keychain integration | 3 | Critical | - |
| 5.4.9 | iOS v2 feature integration | 5 | Critical | - |
| 5.4.10 | iOS UI updates | 3 | High | - |
| 5.4.11 | Mobile performance optimization | 5 | High | - |
| 5.4.12 | Mobile integration tests | 5 | Critical | - |

**Acceptance Criteria:**
- [ ] Android client uses v2 protocol
- [ ] iOS client uses v2 protocol
- [ ] Hybrid crypto works on mobile (performance acceptable)
- [ ] Keystore/Keychain integration updated
- [ ] Push notifications work with v2

**Mobile-Specific Considerations:**

**Android:**
- JNI bindings regenerated for v2 API
- ML-KEM performance on ARM acceptable (<5ms)
- Battery impact of per-packet ratchet acceptable
- Background service uses v2 protocol

**iOS:**
- UniFFI bindings regenerated for v2 API
- Secure Enclave for classical keys (ML-KEM in app)
- Background fetch uses v2 protocol
- App Extensions updated

**Code Location:** `clients/wraith-android/`, `clients/wraith-ios/`

---

### Sprint 5.5: API Compatibility & Shims (13-16 SP)

**Goal:** Provide compatibility shims for gradual migration.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 5.5.1 | Create v1 compat module | 3 | High | - |
| 5.5.2 | Implement Session::new shim | 2 | High | - |
| 5.5.3 | Implement sync send shim | 2 | High | - |
| 5.5.4 | Add deprecation warnings | 2 | High | - |
| 5.5.5 | Feature flag configuration | 2 | High | - |
| 5.5.6 | Shim documentation | 2 | Medium | - |
| 5.5.7 | Shim test coverage | 3 | High | - |

**Acceptance Criteria:**
- [ ] v1 API available under `v1_compat` feature
- [ ] Clear deprecation warnings at compile time
- [ ] Shims correctly delegate to v2 implementation
- [ ] Migration path documented

**Compatibility Shims:**
```rust
#[cfg(feature = "v1-compat")]
pub mod v1_compat {
    #[deprecated(since = "3.0.0", note = "Use Session::builder() instead")]
    pub fn session_new(
        cid: u64,
        addr: SocketAddr,
        secret: [u8; 32],
    ) -> Result<Session> {
        Session::builder()
            .connection_id(ConnectionId::from_v1(cid))
            .peer_addr(addr)
            .crypto_context(CryptoContext::from_classical(secret))
            .v1_compat(true)
            .build()
    }
}
```

**Code Location:** `crates/*/src/compat/`

---

### Sprint 5.6: Future Client Preparation (22-21 SP)

**Goal:** Prepare stubs and architecture for future clients.

#### Tasks

| ID | Task | SP | Priority | Assignee |
|----|------|-----|----------|----------|
| 5.6.1 | wraith-share project scaffold | 3 | Low | - |
| 5.6.2 | wraith-stream project scaffold | 3 | Low | - |
| 5.6.3 | wraith-mesh project scaffold | 3 | Low | - |
| 5.6.4 | wraith-publish project scaffold | 3 | Low | - |
| 5.6.5 | wraith-vault project scaffold | 3 | Low | - |
| 5.6.6 | Shared client library design | 5 | Medium | - |
| 5.6.7 | Client architecture documentation | 2 | Medium | - |

**Acceptance Criteria:**
- [ ] Project scaffolds created with v2 dependencies
- [ ] Shared library patterns established
- [ ] Architecture documented for future development
- [ ] No implementation required (stubs only)

**Code Location:** `clients/wraith-share/`, etc.

---

## Technical Specifications

### API Migration Summary

| v1 API | v2 API | Migration |
|--------|--------|-----------|
| `Session::new(cid, addr, secret)` | `Session::builder()...build()` | Builder pattern |
| `session.send(data)` | `session.send(data).await` | Async |
| `ConnectionId(u64)` | `ConnectionId([u8; 16])` | 128-bit |
| `X25519KeyPair` | `HybridKeyPair` | Hybrid crypto |
| `hkdf_sha256()` | `hkdf_blake3()` | New KDF |

### Client Test Requirements

| Client | Unit Tests | Integration | E2E |
|--------|-----------|-------------|-----|
| wraith-cli | 50+ | 20+ | 10+ |
| wraith-transfer | 80+ | 30+ | 15+ |
| wraith-chat | 90+ | 40+ | 20+ |
| wraith-sync | 30+ | 20+ | 10+ |
| wraith-android | 110+ | 50+ | 20+ |
| wraith-ios | 120+ | 50+ | 20+ |

---

## Testing Requirements

### Test Categories

| Category | Target | Method |
|----------|--------|--------|
| Unit | 80% coverage | Standard framework |
| Integration | All clients | Cross-client tests |
| E2E | All platforms | Full stack tests |
| Mobile | Device farm | Real device testing |

### Test Matrix

```
Client Test Matrix:
═══════════════════

                    v1 Server    v2 Server    v2 Server
                    (compat)     (compat)     (strict)
wraith-cli v2         Pass         Pass         Pass
wraith-transfer v2    Pass         Pass         Pass
wraith-chat v2        Pass         Pass         Pass
wraith-android v2     Pass         Pass         Pass
wraith-ios v2         Pass         Pass         Pass
```

---

## Dependencies

### Internal Dependencies

| Dependency | Type | Notes |
|------------|------|-------|
| wraith-ffi | Foundation | Updated first |
| wraith-core | Consumer | v2 types |
| wraith-crypto | Consumer | Hybrid crypto |
| wraith-transport | Consumer | Multi-transport |

### External Dependencies

| Dependency | Platform | Purpose |
|------------|----------|---------|
| Tauri 2.0 | Desktop | App framework |
| Jetpack Compose | Android | UI |
| SwiftUI | iOS | UI |
| cargo-ndk | Android | JNI build |
| uniffi-rs | iOS | Swift bindings |

---

## Risk Assessment

### High Risk

| Risk | Mitigation |
|------|------------|
| Mobile performance issues | Early benchmarking |
| API breaking changes cascade | Compat shims |
| Platform-specific bugs | Comprehensive CI matrix |

### Medium Risk

| Risk | Mitigation |
|------|------------|
| UI/UX complexity | Incremental UI updates |
| Test coverage gaps | Coverage enforcement |

---

## Deliverables Checklist

### Code Deliverables

- [ ] `crates/wraith-ffi/` - Updated FFI layer
- [ ] `crates/wraith-cli/` - Updated CLI
- [ ] `clients/wraith-transfer/` - Updated Transfer client
- [ ] `clients/wraith-chat/` - Updated Chat client
- [ ] `clients/wraith-sync/` - Updated Sync client
- [ ] `clients/wraith-android/` - Updated Android client
- [ ] `clients/wraith-ios/` - Updated iOS client
- [ ] `crates/*/src/compat/` - Compatibility shims

### Test Deliverables

- [ ] FFI test suite (all platforms)
- [ ] CLI regression tests
- [ ] Desktop client integration tests
- [ ] Mobile client integration tests
- [ ] Cross-platform E2E tests

### Documentation Deliverables

- [ ] FFI migration guide
- [ ] Client migration guide
- [ ] API compatibility notes
- [ ] Mobile-specific considerations

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial Phase 5 sprint plan |
