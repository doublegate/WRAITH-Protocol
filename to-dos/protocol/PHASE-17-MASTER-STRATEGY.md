# Phase 17 Master Strategy and Sprint Plan

**Version:** 1.0.0
**Created:** 2026-01-21
**Status:** Planning (Ready for Implementation)
**Prerequisites:** Phase 16 Complete (v1.6.2) - Verified by [PHASE-16-COMPLETION-AUDIT.md](../completed/PHASE-16-COMPLETION-AUDIT.md)
**Estimated Duration:** 17-23 weeks
**Estimated Story Points:** 280-370 SP

---

## Executive Summary

### Vision

Phase 17 represents the maturation of WRAITH Protocol's client ecosystem, transforming placeholder mobile implementations into fully functional protocol-integrated applications while expanding WRAITH-Chat from text-only messaging to a comprehensive secure communications platform with voice, video, and advanced group messaging capabilities.

### Goals

1. **Mobile Protocol Integration:** Replace placeholder implementations in Android and iOS clients with full WRAITH protocol functionality
2. **Push Notifications:** Enable real-time message delivery on mobile platforms via FCM (Android) and APNs (iOS)
3. **Voice/Video Calls:** Add encrypted real-time communication to WRAITH-Chat using Opus and AV1/VP9 codecs
4. **Group Messaging Enhancements:** Implement multi-party Double Ratchet with sender keys for efficient group encryption

### Scope Overview

| Category | Scope Items | Story Points | Duration |
|----------|-------------|--------------|----------|
| Mobile Protocol Integration | Android FFI, iOS UniFFI, secure storage, discovery | 80-100 | 4-6 weeks |
| Push Notifications | FCM, APNs, background handling | 30-40 | 2-3 weeks |
| Voice/Video Calls | Opus, AV1/VP9, call signaling, quality adaptation | 100-130 | 6-8 weeks |
| Group Messaging | Multi-party DR, sender keys, admin controls | 50-70 | 3-4 weeks |
| Integration Testing | E2E testing, cross-platform, benchmarks | 20-30 | 2 weeks |
| **Total** | | **280-370** | **17-23 weeks** |

### Expected Outcomes

- Mobile clients connect using actual WRAITH protocol (not placeholders)
- Push notifications work reliably on both platforms
- Voice calls with <200ms latency
- Video calls at 720p/30fps minimum
- Group chats support 100+ members with efficient key management
- All existing tests pass (1,630+ tests)
- New functionality has >80% test coverage

---

## Source Document References

This master strategy consolidates and synthesizes content from the following source documents. **Future Claude Code sessions should reference these documents for detailed context.**

### Primary Planning Documents

| Document | Path | Contribution | Priority |
|----------|------|--------------|----------|
| **Phase 17 Planning** | `to-dos/protocol/PHASE-17-PLANNING.md` | Initial feature breakdown, sprint structure, success criteria | Essential |
| **Client Roadmap** | `to-dos/ROADMAP-clients.md` | Comprehensive client specifications, story points, testing requirements | Essential |
| **Protocol Roadmap** | `to-dos/ROADMAP.md` | Project timeline, phase dependencies, performance targets | Essential |
| **Phase 16 Completion Audit** | `to-dos/completed/PHASE-16-COMPLETION-AUDIT.md` | Current state verification, tech debt status, readiness confirmation | Essential |

### Technical Debt References

| Document | Path | Contribution |
|----------|------|--------------|
| **Tech Debt v1.6.1** | `to-dos/technical-debt/TECH-DEBT-v1.6.1.md` | Outstanding items (TD-002 to TD-013), remediation priorities |
| **Tech Debt v1.5.0** | `to-dos/technical-debt/TECH-DEBT-v1.5.0-ARCHIVED.md` | Historical context, resolved items |

### Client Sprint Planning

| Document | Path | Contribution |
|----------|------|--------------|
| **WRAITH-Chat Sprints** | `to-dos/clients/wraith-chat-sprints.md` | Detailed chat implementation plan, Double Ratchet specs, UI patterns |
| **WRAITH-Transfer Sprints** | `to-dos/clients/wraith-transfer-sprints.md` | Reference implementation for Tauri client architecture |

### Architecture Documentation

| Document | Path | Contribution |
|----------|------|--------------|
| **Client Overview** | `docs/clients/overview.md` | Client ecosystem architecture, integration patterns |
| **WRAITH-Chat Architecture** | `docs/clients/wraith-chat/architecture.md` | E2EE implementation details, database schema, security model |

### Completed Phase References (Sprint Pattern Analysis)

| Document | Path | Contribution |
|----------|------|--------------|
| **Phase 16 Summary** | `to-dos/completed/PHASE-16-SUMMARY.md` | Recent completion patterns, lessons learned |
| **Phase 13 Summary** | `to-dos/completed/PHASE-13-SESSION-SUMMARY.md` | Connection management patterns |
| **Phase 12 Complete** | `to-dos/completed/PHASE-12-COMPLETE.md` | Quality gate patterns |

### Document Relationships

```
PHASE-17-MASTER-STRATEGY.md (this document)
    |
    +-- PHASE-17-PLANNING.md (initial breakdown)
    |       |
    |       +-- ROADMAP-clients.md (client specifications)
    |       +-- ROADMAP.md (project timeline)
    |
    +-- TECH-DEBT-v1.6.1.md (outstanding items)
    |       |
    |       +-- Mobile: TD-002, TD-003, TD-004, TD-005, TD-006
    |       +-- Chat: TD-007-TD-013 (mostly resolved)
    |
    +-- wraith-chat-sprints.md (detailed chat specs)
    |       |
    |       +-- Double Ratchet implementation
    |       +-- Voice/video call specifications
    |
    +-- PHASE-16-COMPLETION-AUDIT.md (current state)
            |
            +-- Verification: 1,630 tests, zero warnings
            +-- Prerequisites confirmed
```

---

## Feature Breakdown

### 1. Mobile Protocol Integration (80-100 SP)

**Current State:** Android and iOS clients have placeholder implementations that build and run but use mock data.

#### 1.1 Android Client Protocol Integration

**Technical Debt Items:** TD-002, TD-003

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| JNI Bindings Enhancement | Replace placeholder with actual wraith-ffi calls via cargo-ndk | 21 | High |
| Noise_XX Handshake | Implement proper handshake on mobile with timeout handling | 13 | High |
| Android Keystore Integration | Secure key storage using Android Keystore API | 8 | Medium |
| Discovery Integration | Connect to WRAITH DHT for peer finding | 13 | High |
| Transfer Tracking (TD-002) | Implement real transfer state tracking | 5 | Medium |
| Unwrap Safety (TD-003) | Replace unwrap() with proper error handling | 5 | Medium |

**Dependencies:**
- wraith-ffi crate (complete)
- cargo-ndk build toolchain

**Files to Modify:**
- `clients/wraith-android/app/src/main/rust/src/lib.rs`
- `clients/wraith-android/app/src/main/java/com/wraith/client/WraithClient.kt`

#### 1.2 iOS Client Protocol Integration

**Technical Debt Items:** TD-004, TD-005, TD-006

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| UniFFI Bindings Enhancement | Replace placeholder with actual wraith-ffi integration | 18 | High |
| Noise_XX Handshake | Implement proper handshake with iOS concurrency model | 13 | High |
| iOS Keychain Integration | Secure key storage using iOS Keychain Services | 8 | Medium |
| Discovery Integration | Connect to WRAITH DHT for peer finding | 13 | High |
| File Size Implementation (TD-004) | Query actual file metadata | 2 | Low |
| Transfer Tracking (TD-005) | Implement real transfer state tracking | 5 | Medium |

**Dependencies:**
- uniffi-rs for Swift bindings
- wraith-ffi crate (complete)

**Files to Modify:**
- `clients/wraith-ios/wraith-swift-ffi/src/lib.rs`
- `clients/wraith-ios/WraithApp/WraithApp/`

### 2. Push Notifications (30-40 SP)

#### 2.1 Android Push (FCM)

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| FCM SDK Integration | Add Firebase Cloud Messaging dependency | 3 | Low |
| Message Handler Service | FirebaseMessagingService implementation | 8 | Medium |
| Notification Channels | Android 8.0+ notification channel configuration | 3 | Low |
| Token Management | Registration token storage and refresh | 5 | Medium |
| Background Data Handling | Process encrypted payloads in background | 8 | High |

**Dependencies:**
- Firebase SDK
- Google Play Services

#### 2.2 iOS Push (APNs)

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| APNs Configuration | Certificates, entitlements, capabilities | 3 | Low |
| Notification Service Extension | Decrypt and display encrypted payloads | 8 | High |
| Background App Refresh | Enable background processing | 5 | Medium |
| Rich Notifications | Support for images, actions, groups | 5 | Medium |
| Silent Push Handling | Trigger background sync without alert | 5 | Medium |

**Dependencies:**
- Apple Developer Account
- Push notification certificates

### 3. Voice/Video Calls (100-130 SP)

**Source:** `to-dos/clients/wraith-chat-sprints.md` Sprint 3

#### 3.1 Voice Calling (40-50 SP)

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| Opus Codec Integration | libopus binding for voice encoding/decoding | 13 | High |
| Voice Stream Protocol | Real-time transport over WRAITH streams | 13 | High |
| Echo Cancellation | WebRTC-style echo cancellation | 8 | High |
| Noise Suppression | RNNoise or similar noise reduction | 5 | Medium |
| Jitter Buffer | Adaptive jitter buffer for variable latency | 8 | High |
| Audio Routing | Speaker/headphone/Bluetooth routing | 5 | Medium |

**Performance Requirements:**
- Latency: <200ms end-to-end
- Bitrate: 64 kbps minimum
- Quality: Clear audio with noise suppression

#### 3.2 Video Calling (45-55 SP)

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| Video Codec Integration | AV1 (dav1d) or VP9 for video encoding/decoding | 13 | High |
| Camera Capture | Platform-specific camera access | 8 | Medium |
| Screen Capture | Optional screen sharing capability | 8 | High |
| Adaptive Bitrate | Bandwidth-based quality adjustment | 13 | High |
| Video Preview | Local video preview before/during calls | 5 | Medium |
| Resolution Scaling | 360p/480p/720p/1080p scaling | 5 | Medium |

**Performance Requirements:**
- Latency: <250ms end-to-end
- Quality: 720p @ 30fps @ 1.5 Mbps
- Adaptive: Scale down to 360p @ 300 kbps on poor connections

#### 3.3 Call Signaling (15-25 SP)

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| ICE Candidate Exchange | WRAITH-based candidate signaling | 8 | High |
| Call State Machine | Ringing, connected, ended states | 5 | Medium |
| TURN Relay Fallback | Relay for symmetric NAT situations | 8 | High |
| Call Quality Metrics | RTT, packet loss, jitter monitoring | 5 | Medium |

**Integration with Existing Systems:**
- Uses `wraith-discovery` NAT traversal (TM-001 signaling documented)
- Uses `wraith-crypto` for E2E call encryption

### 4. Group Messaging Enhancements (50-70 SP)

**Source:** `to-dos/clients/wraith-chat-sprints.md` Sprint 2

#### 4.1 Multi-Party Double Ratchet (25-35 SP)

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| Sender Keys Protocol | Efficient group encryption using sender keys | 13 | Very High |
| Key Distribution | Secure sender key distribution to members | 8 | High |
| Group Key Rotation | Periodic key rotation for forward secrecy | 8 | High |
| Member Add/Remove | Re-key on membership changes | 8 | High |

**Security Properties:**
- Forward secrecy within groups
- Post-compromise security via rotation
- Efficient O(1) encryption vs O(n) with pairwise

#### 4.2 Group Administration (15-20 SP)

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| Admin Roles | Admin vs member permission levels | 5 | Medium |
| Member Management | Add, remove, ban, unban operations | 5 | Medium |
| Group Settings | Name, avatar, description, link sharing | 5 | Medium |
| Admin Transfer | Transfer admin role to another member | 3 | Low |

#### 4.3 Group Synchronization (10-15 SP)

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| State Sync Protocol | Ensure consistent group state across devices | 8 | High |
| Conflict Resolution | Handle concurrent membership changes | 5 | Medium |
| Offline Member Handling | Queue messages for offline members | 5 | Medium |

### 5. Integration Testing (20-30 SP)

| Feature | Description | Story Points | Complexity |
|---------|-------------|--------------|------------|
| End-to-End Mobile Testing | Full flow testing on real devices | 8 | High |
| Cross-Platform Interoperability | Desktop-mobile, Android-iOS communication | 8 | High |
| Performance Benchmarks | Latency, throughput, battery consumption | 8 | Medium |
| Security Validation | Cryptographic correctness verification | 5 | High |

---

## Dependency Analysis

### Technical Dependencies

#### External Dependencies (Libraries/Services)

| Dependency | Purpose | Version | Risk |
|------------|---------|---------|------|
| cargo-ndk | Android FFI compilation | Latest | Low |
| uniffi-rs | iOS Swift binding generation | 0.25+ | Low |
| libopus | Voice codec | 1.4+ | Low (well-established) |
| dav1d / libvpx | Video codec (AV1/VP9) | Latest | Medium (codec licensing) |
| Firebase SDK | Android push notifications | Latest | Low |
| APNs | iOS push notifications | System | Low |

#### Internal Dependencies (WRAITH Crates)

| Crate | Required Features | Status |
|-------|-------------------|--------|
| wraith-ffi | FFI-safe types, C API | Complete |
| wraith-core | Node lifecycle, session management | Complete |
| wraith-crypto | Noise_XX, X25519, AEAD | Complete |
| wraith-discovery | DHT, NAT traversal, STUN | Complete |
| wraith-transport | UDP, connection pooling | Complete |
| wraith-obfuscation | Timing, padding (for calls) | Complete |

### Sprint Dependencies

```
Sprint 17.1 (Mobile FFI)
    |
    +-- Sprint 17.2 (Mobile Secure Storage)
    |       |
    |       +-- Sprint 17.3 (Mobile Discovery)
    |               |
    |               +-- Sprint 17.4 (Push Notifications)
    |                       |
    |                       +-- Sprint 17.8 (Integration Testing)
    |
Sprint 17.5 (Voice Calling)
    |
    +-- Sprint 17.6 (Video Calling)
            |
            +-- Sprint 17.8 (Integration Testing)

Sprint 17.7 (Group Messaging) -- Independent, can run in parallel
    |
    +-- Sprint 17.8 (Integration Testing)
```

### Blocking vs Non-Blocking

| Sprint | Blocks | Blocked By |
|--------|--------|------------|
| 17.1 (Mobile FFI) | 17.2, 17.3 | None |
| 17.2 (Secure Storage) | 17.3 | 17.1 |
| 17.3 (Discovery) | 17.4 | 17.2 |
| 17.4 (Push) | 17.8 | 17.3 |
| 17.5 (Voice) | 17.6 | None |
| 17.6 (Video) | 17.8 | 17.5 |
| 17.7 (Groups) | 17.8 | None (parallel) |
| 17.8 (Testing) | None | 17.4, 17.6, 17.7 |

---

## Optimized Sprint Plan

### Sprint Overview

| Sprint | Name | Duration | Story Points | Dependencies |
|--------|------|----------|--------------|--------------|
| 17.1 | Mobile FFI Integration | 2 weeks | 39-52 | None |
| 17.2 | Mobile Secure Storage | 2 weeks | 16-21 | 17.1 |
| 17.3 | Mobile Discovery Integration | 2 weeks | 26-34 | 17.2 |
| 17.4 | Push Notifications | 2 weeks | 30-40 | 17.3 |
| 17.5 | Voice Calling | 3 weeks | 40-50 | None (parallel track) |
| 17.6 | Video Calling | 3 weeks | 45-55 | 17.5 |
| 17.7 | Group Messaging | 3 weeks | 50-70 | None (parallel track) |
| 17.8 | Integration Testing | 2 weeks | 20-30 | 17.4, 17.6, 17.7 |

### Parallel Execution Tracks

```
Week:  1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19
Track A (Mobile):
       [====17.1====][====17.2====][====17.3====][====17.4====]
Track B (Voice/Video):
       [======17.5======][======17.6======]
Track C (Groups):
       [==========17.7==========]
Track D (Testing):
                                                               [====17.8====]
```

**Optimal Duration with Parallelization:** 17-19 weeks (vs 21-23 sequential)

---

### Sprint 17.1: Mobile FFI Integration (2 weeks)

**Goal:** Replace placeholder implementations with actual WRAITH protocol bindings.

**Story Points:** 39-52

#### Tasks

| ID | Task | Points | Platform | Acceptance Criteria |
|----|------|--------|----------|---------------------|
| 17.1.1 | Android JNI binding enhancement | 13 | Android | All wraith-ffi functions callable from Kotlin |
| 17.1.2 | iOS UniFFI binding enhancement | 13 | iOS | Swift async calls to wraith-ffi work correctly |
| 17.1.3 | Shared FFI test suite | 8 | Both | Cross-platform test suite validates bindings |
| 17.1.4 | Error handling standardization | 5 | Both | Platform-appropriate error types returned |

#### Deliverables
- [ ] Android: JNI functions call real wraith-ffi APIs
- [ ] iOS: UniFFI bindings generate working Swift interfaces
- [ ] Shared test suite validates both platforms
- [ ] Error handling follows platform conventions

#### Verification
```bash
# Android
cd clients/wraith-android && ./gradlew test

# iOS
cd clients/wraith-ios && swift test
```

#### Source Documents
- `to-dos/technical-debt/TECH-DEBT-v1.6.1.md` (TD-002, TD-003, TD-004, TD-005, TD-006)
- `to-dos/clients/wraith-chat-sprints.md` (patterns for Tauri integration)

---

### Sprint 17.2: Mobile Secure Storage (2 weeks)

**Goal:** Implement platform-native secure key storage.

**Story Points:** 16-21

**Prerequisites:** Sprint 17.1 complete

#### Tasks

| ID | Task | Points | Platform | Acceptance Criteria |
|----|------|--------|----------|---------------------|
| 17.2.1 | Android Keystore integration | 8 | Android | Keys stored in hardware-backed Keystore |
| 17.2.2 | iOS Keychain integration | 8 | iOS | Keys stored in Secure Enclave where available |
| 17.2.3 | Key migration utilities | 5 | Both | Migrate keys from old storage seamlessly |

#### Deliverables
- [ ] Android: Identity keys stored in Android Keystore
- [ ] iOS: Identity keys stored in iOS Keychain
- [ ] Migration path from any existing key storage

#### Technical Notes
- Use Android Keystore with `PURPOSE_SIGN | PURPOSE_VERIFY` for Ed25519
- Use iOS Keychain with `kSecAttrAccessibleAfterFirstUnlock` for background access
- Reference: TD-008 (chat database key handling) pattern

#### Source Documents
- `to-dos/technical-debt/TECH-DEBT-v1.6.1.md` (TD-008 secure storage pattern)
- `docs/clients/wraith-chat/architecture.md` (keyring integration)

---

### Sprint 17.3: Mobile Discovery Integration (2 weeks)

**Goal:** Connect mobile clients to WRAITH DHT for peer discovery.

**Story Points:** 26-34

**Prerequisites:** Sprint 17.2 complete

#### Tasks

| ID | Task | Points | Platform | Acceptance Criteria |
|----|------|--------|----------|---------------------|
| 17.3.1 | DHT client on Android | 13 | Android | Can discover peers via DHT lookup |
| 17.3.2 | DHT client on iOS | 13 | iOS | Can discover peers via DHT lookup |
| 17.3.3 | NAT traversal for mobile | 8 | Both | Works on cellular networks with aggressive NAT |
| 17.3.4 | Connection keep-alive | 5 | Both | Maintains connections during app backgrounding |

#### Deliverables
- [ ] Mobile clients can find peers via DHT
- [ ] NAT traversal works on typical mobile networks
- [ ] Connections survive brief backgrounding

#### Technical Notes
- Mobile networks often have more aggressive NAT than home networks
- May need shorter keep-alive intervals (30s vs 60s)
- Battery optimization considerations for keep-alive

#### Source Documents
- `to-dos/ROADMAP.md` (Phase 5 NAT traversal)
- `to-dos/technical-debt/TECH-DEBT-v1.6.1.md` (TM-001 NAT signaling)

---

### Sprint 17.4: Push Notifications (2 weeks)

**Goal:** Enable real-time message delivery on mobile platforms.

**Story Points:** 30-40

**Prerequisites:** Sprint 17.3 complete

#### Tasks

| ID | Task | Points | Platform | Acceptance Criteria |
|----|------|--------|----------|---------------------|
| 17.4.1 | FCM integration | 13 | Android | Push notifications delivered reliably |
| 17.4.2 | APNs integration | 13 | iOS | Push notifications delivered reliably |
| 17.4.3 | Background message handling | 8 | Both | Encrypted payloads processed in background |
| 17.4.4 | Notification customization | 5 | Both | Rich notifications with actions |

#### Deliverables
- [ ] Android: FCM push notifications work
- [ ] iOS: APNs push notifications work
- [ ] Background sync triggered by silent push
- [ ] User-facing notifications appear correctly

#### Architecture Decision: Push Notification Server

**Options:**
1. **Self-hosted relay server** - Most private, requires infrastructure
2. **Minimal cloud relay** - Only delivers encrypted wake-up signals
3. **P2P with relay fallback** - No central server, relay when offline

**Recommendation:** Option 2 (minimal cloud relay) for initial implementation
- Sends only opaque "wake up" signal
- Actual message fetched via WRAITH protocol
- No message content on push infrastructure

#### Source Documents
- `to-dos/clients/wraith-chat-sprints.md` (Sprint 4 push notifications)
- `to-dos/ROADMAP-clients.md` (mobile client requirements)

---

### Sprint 17.5: Voice Calling (3 weeks)

**Goal:** Implement encrypted voice calls over WRAITH protocol.

**Story Points:** 40-50

**Prerequisites:** None (parallel track)

#### Tasks

| ID | Task | Points | Platform | Acceptance Criteria |
|----|------|--------|----------|---------------------|
| 17.5.1 | Opus codec integration | 13 | All | Opus encoding/decoding works |
| 17.5.2 | Voice stream protocol | 13 | All | Real-time voice over WRAITH streams |
| 17.5.3 | Echo cancellation | 8 | All | No echo on speakerphone calls |
| 17.5.4 | Noise suppression | 5 | All | Background noise filtered |
| 17.5.5 | Audio routing | 5 | Mobile | Speaker/headphone/Bluetooth switching |

#### Deliverables
- [ ] Voice calls work between desktop and mobile
- [ ] Latency <200ms end-to-end
- [ ] Echo cancellation effective
- [ ] Audio quality acceptable at 64kbps

#### Performance Targets

| Metric | Target | Stretch Goal |
|--------|--------|--------------|
| End-to-end latency | <200ms | <150ms |
| Bitrate | 64 kbps | 48 kbps (low bandwidth mode) |
| Jitter tolerance | 50ms | 100ms |
| Packet loss tolerance | 5% | 10% |

#### Source Documents
- `to-dos/clients/wraith-chat-sprints.md` (Sprint 3 voice/video)
- `to-dos/ROADMAP-clients.md` (WRAITH-Chat performance targets)

---

### Sprint 17.6: Video Calling (3 weeks)

**Goal:** Add video calling capability to WRAITH-Chat.

**Story Points:** 45-55

**Prerequisites:** Sprint 17.5 complete

#### Tasks

| ID | Task | Points | Platform | Acceptance Criteria |
|----|------|--------|----------|---------------------|
| 17.6.1 | Video codec integration | 13 | All | AV1/VP9 encoding/decoding works |
| 17.6.2 | Camera capture | 8 | All | Camera access and preview working |
| 17.6.3 | Screen capture | 8 | Desktop | Screen sharing functional |
| 17.6.4 | Adaptive bitrate | 13 | All | Quality adjusts to bandwidth |
| 17.6.5 | Call UI | 8 | All | Professional call interface |

#### Deliverables
- [ ] Video calls work at 720p/30fps
- [ ] Screen sharing works on desktop
- [ ] Quality adapts to network conditions
- [ ] UI shows local preview, remote video, controls

#### Performance Targets

| Metric | Target | Stretch Goal |
|--------|--------|--------------|
| Resolution | 720p @ 30fps | 1080p @ 30fps |
| Bitrate | 1.5 Mbps | 3 Mbps (HD mode) |
| Low bandwidth mode | 360p @ 300kbps | 240p @ 150kbps |
| Startup time | <3 seconds | <1.5 seconds |

#### Source Documents
- `to-dos/clients/wraith-chat-sprints.md` (Sprint 3 voice/video)

---

### Sprint 17.7: Group Messaging (3 weeks)

**Goal:** Enhance WRAITH-Chat with efficient group encryption and administration.

**Story Points:** 50-70

**Prerequisites:** None (parallel track)

#### Tasks

| ID | Task | Points | Platform | Acceptance Criteria |
|----|------|--------|----------|---------------------|
| 17.7.1 | Multi-party Double Ratchet | 13 | All | Sender keys protocol implemented |
| 17.7.2 | Group key distribution | 8 | All | Secure key exchange with members |
| 17.7.3 | Group key rotation | 8 | All | Automatic rotation every 7 days or member change |
| 17.7.4 | Admin controls | 5 | All | Admin role with elevated permissions |
| 17.7.5 | Member management | 5 | All | Add/remove/ban operations |
| 17.7.6 | Group settings sync | 5 | All | Name, avatar, description synchronized |
| 17.7.7 | Conflict resolution | 5 | All | Consistent state across devices |

#### Deliverables
- [ ] Group messages encrypted with sender keys
- [ ] Groups support 100+ members
- [ ] Key rotation happens automatically
- [ ] Admin operations work correctly

#### Security Properties

| Property | Implementation |
|----------|----------------|
| Forward secrecy | Key rotation invalidates old keys |
| Post-compromise security | New keys after member removal |
| Efficiency | O(1) encryption with sender keys |
| Member removal | Immediate key rotation |

#### Source Documents
- `to-dos/clients/wraith-chat-sprints.md` (Sprint 2 group messaging)

---

### Sprint 17.8: Integration Testing (2 weeks)

**Goal:** Validate all Phase 17 features work together across platforms.

**Story Points:** 20-30

**Prerequisites:** Sprints 17.4, 17.6, 17.7 complete

#### Tasks

| ID | Task | Points | Platform | Acceptance Criteria |
|----|------|--------|----------|---------------------|
| 17.8.1 | End-to-end mobile testing | 8 | Mobile | Full flow on real devices |
| 17.8.2 | Cross-platform interop | 8 | All | Desktop-mobile, Android-iOS work |
| 17.8.3 | Performance benchmarks | 8 | All | Meet all latency/throughput targets |
| 17.8.4 | Security validation | 5 | All | Cryptographic correctness verified |

#### Deliverables
- [ ] All features work across all platforms
- [ ] Performance targets met
- [ ] No security regressions
- [ ] Test coverage >80% for new code

#### Test Matrix

| Feature | Android | iOS | Desktop | Cross-Platform |
|---------|---------|-----|---------|----------------|
| Protocol Connection | Test | Test | Test | Android-Desktop |
| Push Notifications | Test | Test | N/A | N/A |
| Voice Calls | Test | Test | Test | All combinations |
| Video Calls | Test | Test | Test | All combinations |
| Group Messaging | Test | Test | Test | All combinations |

#### Source Documents
- `to-dos/ROADMAP-clients.md` (testing requirements)
- `to-dos/completed/PHASE-16-COMPLETION-AUDIT.md` (testing patterns)

---

## Technical Architecture Notes

### Mobile Architecture Patterns

#### FFI Layer Design

```
Mobile App (Kotlin/Swift)
    |
    +-- FFI Wrapper (type conversion, error handling)
    |       |
    |       +-- wraith-ffi (C-compatible API)
    |               |
    |               +-- wraith-core (Node, Session)
    |               +-- wraith-crypto (Noise, AEAD)
    |               +-- wraith-discovery (DHT, NAT)
```

#### Async Pattern for Mobile

```kotlin
// Android: Use coroutines with FFI
suspend fun establishSession(peerId: String): SessionInfo {
    return withContext(Dispatchers.IO) {
        WraithFFI.establishSession(peerId)
    }
}
```

```swift
// iOS: Use async/await with UniFFI
func establishSession(peerId: String) async throws -> SessionInfo {
    try await wraithNode.establishSession(peerId: peerId)
}
```

### Voice/Video Architecture

#### Call Stack

```
Application Layer (UI, call management)
    |
    +-- Call Manager (signaling, state machine)
    |       |
    |       +-- Audio Engine (Opus, echo cancel)
    |       |       |
    |       |       +-- WRAITH Stream (encrypted transport)
    |       |
    |       +-- Video Engine (AV1/VP9, adaptive bitrate)
    |               |
    |               +-- WRAITH Stream (encrypted transport)
    |
    +-- ICE/STUN (via wraith-discovery)
```

#### Codec Selection

| Codec | Use Case | Licensing |
|-------|----------|-----------|
| Opus | Voice | BSD (open source) |
| AV1 | Video (preferred) | Royalty-free |
| VP9 | Video (fallback) | Royalty-free |

### Group Messaging Architecture

#### Sender Keys Protocol

```
Group with N members:

Traditional Pairwise:
- Sender encrypts message N times (one per member)
- O(N) encryption operations

Sender Keys:
- Each member has a "sender key" distributed to group
- Sender encrypts message once with their sender key
- O(1) encryption operations
```

#### Key Rotation Triggers

1. **Time-based:** Every 7 days
2. **Member removal:** Immediate rotation
3. **Suspected compromise:** Manual trigger
4. **Message count:** Every 1,000 messages (optional)

---

## Risk Assessment

### Technical Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Mobile platform API changes | Medium | Medium | Pin SDK versions, maintain compatibility layer |
| Codec licensing issues | Low | Low | Use open source implementations (libopus, dav1d) |
| Battery consumption on mobile | Medium | Medium | Optimize background processing, adaptive polling |
| Network reliability on mobile | Medium | High | Robust reconnection, message queuing |
| NAT traversal on mobile networks | High | Medium | TURN relay fallback, birthday attack for symmetric NAT |

### Schedule Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Voice/video complexity underestimated | High | Medium | Use existing WebRTC patterns, start with MVP |
| Platform-specific bugs | Medium | High | Early cross-platform testing, platform experts |
| Integration issues between tracks | Medium | Medium | Regular integration builds, shared interfaces |

### Resource Constraints

| Constraint | Impact | Mitigation |
|------------|--------|------------|
| Limited real device access | Testing gaps | Use emulators + limited real device testing |
| No dedicated mobile developer | Slower progress | Rust-first approach, minimal platform code |
| Push notification infrastructure | Delayed push feature | Use minimal relay approach |

### Risk Mitigation Strategies

1. **Technical Risks:**
   - Start with MVP implementations, iterate
   - Use well-established libraries (Opus, dav1d)
   - Maintain fallback modes (TURN relay, lower quality)

2. **Schedule Risks:**
   - Parallel tracks to reduce critical path
   - Feature flags for incremental rollout
   - Regular integration testing

3. **Resource Risks:**
   - Prioritize Rust-based implementation
   - Leverage existing wraith-* crate patterns
   - Document decisions for future developers

---

## Success Criteria

### Phase 17 Completion Checklist

#### Mobile Protocol Integration
- [ ] Mobile clients connect using actual WRAITH protocol
- [ ] Android: JNI bindings fully functional
- [ ] iOS: UniFFI bindings fully functional
- [ ] Secure key storage on both platforms
- [ ] DHT peer discovery working on mobile

#### Push Notifications
- [ ] FCM push notifications work on Android
- [ ] APNs push notifications work on iOS
- [ ] Background message sync functional
- [ ] No plaintext on push infrastructure

#### Voice/Video Calls
- [ ] Voice calls: <200ms latency
- [ ] Video calls: 720p @ 30fps
- [ ] Echo cancellation effective
- [ ] Adaptive bitrate working
- [ ] Screen sharing on desktop

#### Group Messaging
- [ ] Sender keys protocol implemented
- [ ] Groups support 100+ members
- [ ] Key rotation automatic
- [ ] Admin controls functional

#### Quality Gates
- [ ] All existing tests pass (1,630+)
- [ ] New functionality has >80% test coverage
- [ ] Zero clippy warnings
- [ ] Security review passed
- [ ] Performance targets met

### Documentation Requirements

- [ ] API documentation for new features
- [ ] User guide for voice/video calls
- [ ] Administrator guide for push notifications
- [ ] Migration guide for group messaging upgrade
- [ ] CHANGELOG updated with all changes

---

## For Claude Code Sessions

### Starting a Phase 17 Sprint

1. **Read this document first** for overall context and strategy
2. **Check sprint dependencies** in the dependency analysis section
3. **Review relevant source documents** listed in the sprint section
4. **Verify current state** by running:
   ```bash
   cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings
   ```

### Sprint-Specific References

| Sprint | Primary Documents | Key Files |
|--------|-------------------|-----------|
| 17.1 (Mobile FFI) | `TECH-DEBT-v1.6.1.md`, existing FFI code | `clients/wraith-android/.../lib.rs`, `clients/wraith-ios/.../lib.rs` |
| 17.2 (Secure Storage) | `TECH-DEBT-v1.6.1.md` (TD-008 pattern) | Platform keystore/keychain code |
| 17.3 (Discovery) | `ROADMAP.md` Phase 5, `TECH-DEBT-v1.6.1.md` | `crates/wraith-discovery/` |
| 17.4 (Push) | `wraith-chat-sprints.md` Sprint 4 | New push notification service |
| 17.5 (Voice) | `wraith-chat-sprints.md` Sprint 3 | New audio engine |
| 17.6 (Video) | `wraith-chat-sprints.md` Sprint 3 | New video engine |
| 17.7 (Groups) | `wraith-chat-sprints.md` Sprint 2 | `clients/wraith-chat/src-tauri/src/crypto.rs` |
| 17.8 (Testing) | All sprint documents | Test files across all platforms |

### Key Technical Decisions to Reference

1. **FFI Pattern:** See `crates/wraith-ffi/src/lib.rs` for established patterns
2. **Tauri IPC:** See `clients/wraith-transfer/src-tauri/src/commands.rs`
3. **Double Ratchet:** See `clients/wraith-chat/src-tauri/src/crypto.rs`
4. **Database:** See `clients/wraith-chat/src-tauri/src/database.rs`

### Pre-Sprint Verification

Before starting any sprint:
```bash
# Verify build
cargo build --workspace

# Run all tests
cargo test --workspace

# Check code quality
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check

# Check current test count
cargo test --workspace 2>&1 | grep -E "(running|test result)"
```

### Post-Sprint Checklist

1. All acceptance criteria met
2. Tests pass locally
3. Documentation updated
4. CHANGELOG entry added
5. Code reviewed
6. Sprint summary written

---

## Appendix: Story Point Estimation Guide

Based on WRAITH Protocol historical data:

| Points | Complexity | Duration | Example |
|--------|------------|----------|---------|
| 2-3 | Low | 0.5-1 day | Bug fix, simple feature |
| 5 | Medium | 1-2 days | Component feature |
| 8 | Medium-High | 2-3 days | Subsystem feature |
| 13 | High | 3-5 days | Complex integration |
| 21+ | Very High | 1+ week | Major subsystem |

---

**Document Version:** 1.0.0
**Last Updated:** 2026-01-21
**Next Review:** After Sprint 17.1 completion
**Related Documents:**
- [PHASE-17-PLANNING.md](./PHASE-17-PLANNING.md)
- [ROADMAP-clients.md](../ROADMAP-clients.md)
- [ROADMAP.md](../ROADMAP.md)
- [PHASE-16-COMPLETION-AUDIT.md](../completed/PHASE-16-COMPLETION-AUDIT.md)
