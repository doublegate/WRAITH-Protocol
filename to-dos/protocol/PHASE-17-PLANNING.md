# Phase 17 Planning - Mobile Integration & Chat Enhancements

**Version:** 1.0.0
**Created:** 2026-01-21
**Status:** Planning
**Prerequisites:** Phase 16 Complete (v1.6.2)

---

## Executive Summary

Phase 17 focuses on completing the WRAITH protocol integration in mobile clients and enhancing WRAITH-Chat with voice/video capabilities and group messaging improvements.

---

## Deferred Items from Phase 16

The following items were identified during Phase 16 completion as requiring dedicated sprints:

### 1. Mobile Client Protocol Integration

**Current State:** Android and iOS clients have placeholder implementations.

**Android Client (`clients/wraith-android/`)**
- Replace placeholder with actual wraith-ffi JNI bindings via cargo-ndk
- Implement proper Noise_XX handshake on mobile
- Add secure key storage using Android Keystore
- Integrate with WRAITH discovery for peer finding

**iOS Client (`clients/wraith-ios/`)**
- Replace placeholder with actual UniFFI bindings
- Implement proper Noise_XX handshake on mobile
- Add secure key storage using iOS Keychain
- Integrate with WRAITH discovery for peer finding

**Estimated Story Points:** 80-100 SP
**Duration:** 4-6 weeks

### 2. Push Notifications for Mobile Apps

**Android:**
- Firebase Cloud Messaging (FCM) integration
- Background service for message delivery
- Notification channels and customization

**iOS:**
- Apple Push Notification Service (APNs) integration
- Background app refresh
- Notification extensions for rich content

**Estimated Story Points:** 30-40 SP
**Duration:** 2-3 weeks

### 3. Voice/Video Calls in WRAITH-Chat

**Audio:**
- Opus codec integration for voice
- Real-time transport over WRAITH streams
- Echo cancellation and noise suppression

**Video:**
- AV1/VP9 codec integration
- Adaptive bitrate streaming
- Camera/screen capture integration

**WebRTC-like Features:**
- ICE candidate exchange via WRAITH
- TURN relay fallback
- Quality adaptation

**Estimated Story Points:** 100-130 SP
**Duration:** 6-8 weeks

### 4. Group Messaging Enhancements

**Protocol:**
- Multi-party Double Ratchet implementation
- Sender keys for efficient group encryption
- Group key rotation

**Features:**
- Admin controls and permissions
- Member management (add/remove/ban)
- Group settings synchronization

**Estimated Story Points:** 50-70 SP
**Duration:** 3-4 weeks

---

## Phase 17 Sprint Breakdown

### Sprint 17.1: Mobile FFI Integration (2 weeks)
- [ ] Android JNI bindings implementation
- [ ] iOS UniFFI bindings implementation
- [ ] Shared FFI test suite

### Sprint 17.2: Mobile Secure Storage (2 weeks)
- [ ] Android Keystore integration
- [ ] iOS Keychain integration
- [ ] Key migration utilities

### Sprint 17.3: Mobile Discovery Integration (2 weeks)
- [ ] WRAITH discovery on mobile
- [ ] NAT traversal for mobile networks
- [ ] Connection keep-alive for mobile

### Sprint 17.4: Push Notifications (2 weeks)
- [ ] FCM integration (Android)
- [ ] APNs integration (iOS)
- [ ] Background message handling

### Sprint 17.5: Voice Calling (3 weeks)
- [ ] Opus codec integration
- [ ] Voice stream protocol
- [ ] Echo/noise processing

### Sprint 17.6: Video Calling (3 weeks)
- [ ] Video codec integration (AV1/VP9)
- [ ] Camera capture integration
- [ ] Adaptive bitrate

### Sprint 17.7: Group Messaging (3 weeks)
- [ ] Multi-party Double Ratchet
- [ ] Group key management
- [ ] Admin controls

### Sprint 17.8: Integration Testing (2 weeks)
- [ ] End-to-end mobile testing
- [ ] Cross-platform interoperability
- [ ] Performance benchmarks

---

## Total Estimates

| Category | Story Points | Duration |
|----------|--------------|----------|
| Mobile Protocol Integration | 80-100 | 4-6 weeks |
| Push Notifications | 30-40 | 2-3 weeks |
| Voice/Video Calls | 100-130 | 6-8 weeks |
| Group Messaging | 50-70 | 3-4 weeks |
| Integration Testing | 20-30 | 2 weeks |
| **Total** | **280-370** | **17-23 weeks** |

---

## Dependencies

### External
- cargo-ndk for Android FFI
- uniffi-rs for iOS bindings
- Opus codec library
- AV1/VP9 codec libraries

### Internal
- wraith-ffi crate (complete)
- wraith-crypto (complete)
- wraith-discovery (complete)
- wraith-transport (complete)

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Mobile platform API changes | Medium | Pin SDK versions, maintain compatibility layer |
| Codec licensing | Low | Use open source implementations (libopus, dav1d) |
| Battery consumption | Medium | Optimize background processing, adaptive polling |
| Network reliability | Medium | Robust reconnection, message queuing |

---

## Success Criteria

- [ ] Mobile clients connect using actual WRAITH protocol (not placeholders)
- [ ] Push notifications work reliably on both platforms
- [ ] Voice calls with <200ms latency
- [ ] Video calls at 720p/30fps
- [ ] Group chats support 100+ members
- [ ] All existing tests pass
- [ ] New functionality has >80% test coverage

---

## Future Considerations (Post-Phase 17)

1. **XDP Support:** wraith-xdp crate for kernel bypass (requires eBPF toolchain)
2. **Post-Quantum Crypto:** Hybrid mode with Kyber/Dilithium
3. **WRAITH-Sync:** Serverless backup synchronization client
4. **WRAITH-Share:** Distributed file sharing client

---

**See Also:**
- [Phase 16 Completion Summary](../completed/PHASE-16-SUMMARY.md)
- [Client Roadmap](../ROADMAP-clients.md)
- [Protocol Roadmap](../ROADMAP.md)
