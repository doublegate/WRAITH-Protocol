# Master Remaining Items Strategy - WRAITH Protocol

**Version:** 1.0.0
**Created:** 2026-01-21
**Status:** Authoritative Development Guide
**Last Updated:** 2026-01-21
**Author:** Claude Code (Opus 4.5)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Phase 18: Existing Client Completion](#phase-18-existing-client-completion)
3. [Phase 19: Infrastructure Enhancements](#phase-19-infrastructure-enhancements)
4. [Phase 20: Tier 2 Client - WRAITH-Share](#phase-20-tier-2-client---wraith-share)
5. [Phase 21+: Tier 3 Client Applications](#phase-21-tier-3-client-applications)
6. [Security Testing Clients](#security-testing-clients)
7. [Document References](#document-references)
8. [Success Criteria and Quality Gates](#success-criteria-and-quality-gates)

---

## Executive Summary

### Current Project State (v1.7.2)

| Metric | Value | Status |
|--------|-------|--------|
| **Version** | 1.7.2 | UI/UX Design System Release |
| **Tests** | 1,695 passing (16 ignored) | 100% pass rate |
| **Code Volume** | ~68,000 lines Rust (protocol) + ~12,000 lines (clients) | 11 crates |
| **Documentation** | 120+ files, 72,000+ lines | Comprehensive |
| **Security** | Zero vulnerabilities | EXCELLENT posture |
| **Technical Debt** | 8 items (down from 25) | 68% reduction |
| **Quality Score** | 98/100 | Zero clippy warnings |

### Production Clients Complete

| Client | Platform | Tests | Status |
|--------|----------|-------|--------|
| WRAITH-Transfer | Desktop (Tauri) | 68 | Complete |
| WRAITH-Chat | Desktop (Tauri) | 76 | Complete |
| WRAITH-Sync | Desktop (Tauri) | 17 | Complete |
| WRAITH-Android | Android (Kotlin) | 96 | Complete |
| WRAITH-iOS | iOS (Swift) | 103 | Complete |

### Total Remaining Work Estimate

| Phase | Focus | Story Points | Priority |
|-------|-------|--------------|----------|
| Phase 18 | Existing Client Completion | 25-35 SP | HIGH |
| Phase 19 | Infrastructure Enhancements | 40-60 SP | ✅ COMPLETE (Sprint 19.2, 19.3) |
| Phase 20 | WRAITH-Share | 104 SP | MEDIUM |
| Phase 21 | WRAITH-Stream | 71 SP | LOW |
| Phase 22 | WRAITH-Mesh | 60 SP | LOW |
| Phase 23 | WRAITH-Publish | 76 SP | LOW |
| Phase 24 | WRAITH-Vault | 94 SP | LOW |
| **Total** | | **~470-500 SP** | |

### Recommended Development Order

1. **Phase 18** (Immediate): Complete WRAITH-Chat wire-up and polish
2. **Phase 20** (Next): WRAITH-Share - highest value Tier 2 client
3. **Phase 21-24** (Future): Tier 3 clients based on user demand
4. **Phase 19** (Deferred): Infrastructure only when needed

---

## Phase 18: Existing Client Completion

**Goal:** Complete all remaining TODO items in existing clients and achieve production polish.

**Duration:** 2-3 weeks
**Total Story Points:** 25-35 SP
**Priority:** HIGH

### Sprint 18.1: WRAITH-Chat Protocol Wire-Up (TD-014)

**Duration:** 1 week
**Story Points:** 13-18 SP
**Status:** Required for full chat functionality

#### Task 18.1.1: UI Event Emission After Session Establishment

**File:** `clients/wraith-chat/src-tauri/src/commands.rs:265`

**Description:** Emit frontend UI events after session establishment to update connection status indicators.

**Implementation:**
```rust
// After session is established successfully:
window.emit("session_established", SessionEstablishedPayload {
    peer_id: peer_id.clone(),
    session_id: session.id(),
    timestamp: SystemTime::now(),
}).ok();
```

**Story Points:** 2 SP
**Acceptance Criteria:**
- [ ] Frontend receives `session_established` event
- [ ] UI updates to show connected state
- [ ] Event includes peer_id and session metadata
- [ ] Error handling for failed events

**Dependencies:** None

---

#### Task 18.1.2: Group Invitations via WRAITH Protocol

**File:** `clients/wraith-chat/src-tauri/src/commands.rs:718`

**Description:** Wire up group invitation sending through WRAITH protocol streams.

**Implementation:**
```rust
// Send group invitation via WRAITH protocol
async fn send_group_invitation(
    node: &WraithNode,
    peer_id: &str,
    invitation: &GroupInvitation,
) -> Result<(), Error> {
    let session = node.get_or_create_session(peer_id).await?;
    let stream = session.open_stream().await?;

    let payload = serde_json::to_vec(&GroupMessage::Invitation(invitation.clone()))?;
    stream.send(&payload).await?;

    Ok(())
}
```

**Story Points:** 3 SP
**Acceptance Criteria:**
- [ ] Group invitations sent via WRAITH encrypted streams
- [ ] Recipient receives invitation notification
- [ ] Invitation includes group metadata and inviter info
- [ ] Error handling for offline peers (queue for later)

**Dependencies:** Task 18.1.1

---

#### Task 18.1.3: Group Messages via WRAITH Protocol

**File:** `clients/wraith-chat/src-tauri/src/commands.rs:991`

**Description:** Wire up group message sending through WRAITH protocol with Sender Keys encryption.

**Implementation:**
```rust
// Send group message via WRAITH protocol
async fn send_group_message(
    node: &WraithNode,
    group_id: &str,
    content: &str,
    sender_keys: &SenderKeys,
) -> Result<(), Error> {
    let group = get_group(group_id)?;

    // Encrypt with Sender Keys (O(1) efficiency)
    let encrypted = sender_keys.encrypt(content.as_bytes())?;

    // Fan out to all group members
    for member in &group.members {
        if member.peer_id != node.peer_id() {
            let session = node.get_or_create_session(&member.peer_id).await?;
            let stream = session.open_stream().await?;
            stream.send(&encrypted).await?;
        }
    }

    Ok(())
}
```

**Story Points:** 5 SP
**Acceptance Criteria:**
- [ ] Group messages encrypted with Sender Keys
- [ ] Messages delivered to all online group members
- [ ] Offline members receive messages on reconnect
- [ ] Message ordering preserved within group
- [ ] Delivery receipts aggregated

**Dependencies:** Task 18.1.2

---

#### Task 18.1.4: Sender Key Distribution via WRAITH Protocol

**File:** `clients/wraith-chat/src-tauri/src/commands.rs:1034`

**Description:** Implement Sender Key distribution when new members join or keys rotate.

**Implementation:**
```rust
// Distribute sender keys to group members
async fn distribute_sender_keys(
    node: &WraithNode,
    group_id: &str,
    sender_keys: &SenderKeys,
) -> Result<(), Error> {
    let group = get_group(group_id)?;

    for member in &group.members {
        if member.peer_id != node.peer_id() {
            let session = node.get_or_create_session(&member.peer_id).await?;
            let stream = session.open_stream().await?;

            // Encrypt sender key with member's public key
            let key_bundle = sender_keys.export_for_member(&member.public_key)?;
            let message = GroupMessage::SenderKeyDistribution(key_bundle);
            stream.send(&serde_json::to_vec(&message)?).await?;
        }
    }

    Ok(())
}
```

**Story Points:** 3 SP
**Acceptance Criteria:**
- [ ] Sender keys distributed to all group members
- [ ] Keys encrypted with each member's public key
- [ ] Key rotation triggers redistribution
- [ ] New members receive current sender key

**Dependencies:** Task 18.1.3

---

#### Task 18.1.5: Audio Device Switching

**File:** `clients/wraith-chat/src-tauri/src/voice_call.rs:596`

**Description:** Implement audio input/output device switching during voice calls.

**Implementation:**
```rust
// Switch audio devices during call
#[tauri::command]
async fn switch_audio_device(
    state: State<'_, AppState>,
    device_type: DeviceType,
    device_id: String,
) -> Result<(), Error> {
    let call_state = state.voice_call.lock().await;

    match device_type {
        DeviceType::Input => {
            call_state.audio_engine.set_input_device(&device_id)?;
        }
        DeviceType::Output => {
            call_state.audio_engine.set_output_device(&device_id)?;
        }
    }

    Ok(())
}

#[tauri::command]
async fn list_audio_devices() -> Result<AudioDevices, Error> {
    let devices = AudioDevices {
        inputs: enumerate_input_devices()?,
        outputs: enumerate_output_devices()?,
    };
    Ok(devices)
}
```

**Story Points:** 2-3 SP
**Acceptance Criteria:**
- [ ] List available input devices (microphones)
- [ ] List available output devices (speakers/headphones)
- [ ] Switch devices without interrupting call
- [ ] Remember device preferences
- [ ] Handle device disconnection gracefully

**Dependencies:** None (independent of group messaging)

---

### Sprint 18.2: Test Coverage Enhancement (TL-002)

**Duration:** 3-5 days
**Story Points:** 5-8 SP
**Priority:** MEDIUM

#### Task 18.2.1: Voice Call Edge Cases

**Description:** Add tests for voice call edge cases not currently covered.

**Test Scenarios:**
- Call during network transition (WiFi to cellular)
- Microphone permission denied mid-call
- Audio device disconnected during call
- Call reconnection after brief network drop
- Echo cancellation verification
- Jitter buffer underflow/overflow

**Story Points:** 3 SP
**Files:** `clients/wraith-chat/src-tauri/src/voice_call.rs`

---

#### Task 18.2.2: Video Call Edge Cases

**Description:** Add tests for video call edge cases.

**Test Scenarios:**
- Camera permission denied
- Screen share on multi-monitor setup
- Resolution change during call
- Bandwidth adaptation under stress
- Frame drop recovery
- Codec fallback (VP9 to VP8)

**Story Points:** 3 SP
**Files:** `clients/wraith-chat/src-tauri/src/video_call.rs`

---

#### Task 18.2.3: Group Messaging Edge Cases

**Description:** Add tests for group messaging edge cases.

**Test Scenarios:**
- Message ordering with concurrent sends
- Sender key rotation during active conversation
- Member join during key distribution
- Offline member key catch-up
- Large group (100+ members) performance
- Admin role transfer

**Story Points:** 2 SP
**Files:** `clients/wraith-chat/src-tauri/src/group.rs`

---

### Sprint 18.3: Quality Polish (TL-003, TL-004)

**Duration:** 3-5 days
**Story Points:** 7-9 SP
**Priority:** LOW

#### Task 18.3.1: Chat Statistics Enhancement (TL-003)

**Description:** Enhance chat statistics with additional metrics.

**Metrics to Add:**
- Messages sent/received per day/week/month
- Average message latency
- Call duration statistics
- Group activity metrics
- Encryption key rotation count
- Storage usage breakdown

**Story Points:** 3 SP
**Files:** `clients/wraith-chat/src-tauri/src/commands.rs`

---

#### Task 18.3.2: Mobile Device Testing (TL-004)

**Description:** Test mobile clients on physical devices.

**Test Matrix:**

| Platform | Devices | Test Focus |
|----------|---------|------------|
| Android | Pixel 6, Samsung S22, older device | Performance, battery, permissions |
| iOS | iPhone 13, iPhone SE, iPad | Performance, Keychain, APNs |

**Test Scenarios:**
- Background sync behavior
- Push notification delivery
- Battery consumption during idle
- App suspension/resume
- Low memory handling
- Network transitions

**Story Points:** 4-6 SP
**Files:** Mobile client test suites

---

## Phase 19: Infrastructure Enhancements

**Goal:** Implement deferred infrastructure items for advanced use cases.

**Duration:** 4-6 weeks (if undertaken)
**Total Story Points:** 40-60 SP
**Priority:** MEDIUM (Deferred - implement when needed)

**Note:** These items are functional with current fallbacks. Only implement when specific use case demands.

### Sprint 19.1: DNS STUN Resolution (TD-001)

**Duration:** 1 week
**Story Points:** 8-12 SP
**Status:** Enhancement - fallback IPs work reliably

#### Task 19.1.1: DNS Resolution for STUN Servers

**Files:**
- `crates/wraith-discovery/src/nat/types.rs:116`
- `crates/wraith-discovery/src/manager.rs:16`
- `crates/wraith-discovery/src/manager.rs:21`

**Description:** Replace hardcoded STUN server IPs with DNS resolution.

**Implementation:**
```rust
// Resolve STUN servers via DNS
async fn resolve_stun_servers(
    hostnames: &[&str],
) -> Result<Vec<SocketAddr>, Error> {
    let mut resolved = Vec::new();

    for hostname in hostnames {
        match tokio::net::lookup_host(hostname).await {
            Ok(addrs) => {
                for addr in addrs {
                    resolved.push(addr);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to resolve {}: {}", hostname, e);
            }
        }
    }

    // Fallback to hardcoded IPs if DNS fails
    if resolved.is_empty() {
        resolved.extend(FALLBACK_STUN_IPS.iter().cloned());
    }

    Ok(resolved)
}
```

**Story Points:** 8-12 SP
**Acceptance Criteria:**
- [ ] DNS resolution with caching
- [ ] Fallback to hardcoded IPs on failure
- [ ] Configurable STUN server list
- [ ] IPv4 and IPv6 support
- [ ] DNS TTL respect

---

### Sprint 19.2: ICE Signaling (TM-001) - COMPLETE

**Duration:** Completed 2026-01-24
**Story Points:** 20-30 SP
**Status:** ✅ COMPLETE

**File:** `crates/wraith-core/src/node/ice.rs`

**Description:** Full ICE (Interactive Connectivity Establishment) implementation per RFC 8445.

**Implemented Components:**
- ✅ IceAgent with role management (Controlling/Controlled)
- ✅ Candidate gathering (Host, Server Reflexive, Peer Reflexive, Relay)
- ✅ Candidate prioritization per RFC 8445 Section 5.1.2
- ✅ CheckList with state management
- ✅ Connectivity checks with nominated pair tracking
- ✅ ICE restart support
- ✅ TURN server integration
- ✅ Comprehensive statistics (IceStats, IceStatsSnapshot)
- ✅ DHT-based signaling for candidate exchange

---

### Sprint 19.3: AF_XDP Implementation (TH-006) - COMPLETE

**Duration:** Completed 2026-01-24
**Story Points:** 30-40 SP
**Status:** ✅ COMPLETE

**File:** `crates/wraith-transport/src/af_xdp.rs`

**Description:** Complete AF_XDP socket implementation with UMEM and ring buffer configuration.

**Implemented Components:**
- ✅ Full socket creation with proper XDP options
- ✅ UMEM allocation with descriptor management
- ✅ RX/TX/Fill/Completion ring buffers
- ✅ Atomic producer/consumer index management
- ✅ Batch receive/transmit operations
- ✅ Comprehensive statistics tracking (AfXdpStats, AfXdpStatsSnapshot)
- ✅ Proper cleanup and resource management

**Requirements for use:**
- Linux kernel 6.2+
- XDP-capable NIC (Intel X710, Mellanox ConnectX-5)
- libbpf and clang toolchain
- Root/CAP_NET_ADMIN privileges

---

## Phase 20: Tier 2 Client - WRAITH-Share

**Goal:** Build distributed anonymous file sharing client with granular access control.

**Duration:** 8 weeks
**Total Story Points:** 104 SP
**Priority:** MEDIUM

**Reference:** [wraith-share-sprints.md](clients/wraith-share-sprints.md)

### Sprint 20.1: Core Sharing Engine (Weeks 1-4)

**Story Points:** 58 SP

| Task | Points | Description |
|------|--------|-------------|
| S1.1 Group Management | 8 | Create, invite, remove members |
| S1.2 Cryptographic Access Control | 13 | Capability-based encryption |
| S1.3 File Upload/Download | 8 | Encrypted transfer with progress |
| S1.4 File Versioning | 8 | Track versions, restore previous |
| S1.5 Activity Log | 5 | Record all file/member events |
| S1.6 Member Invitation Flow | 5 | QR code, link-based invites |
| S1.7 Link Sharing | 8 | Public links with expiration |
| S1.8 CLI Testing Interface | 3 | Command-line tool for testing |

**Key Deliverables:**
- Group creation and management
- Permission system (read, write, admin roles)
- Cryptographic access control (capability-based)
- File upload/download with encryption
- Activity log tracking

---

### Sprint 20.2: GUI and Distribution (Weeks 5-8)

**Story Points:** 46 SP

| Task | Points | Description |
|------|--------|-------------|
| S2.1 Tauri Desktop GUI | 13 | File browser, member list, permissions |
| S2.2 React PWA | 13 | Web-based file access |
| S2.3 File Search | 5 | Full-text search across files |
| S2.4 Bulk Operations | 5 | Multi-select upload/download/delete |
| S2.5 Admin Dashboard | 8 | Member management, audit log |
| S2.6 Offline PWA Support | 5 | Service worker caching |
| S2.7 Platform Installers | 2 | Desktop builds |
| S2.8 PWA Deployment | 1 | Static hosting, manifest |

**Success Criteria:**
- [ ] Groups with 100+ members functional
- [ ] Permissions enforced cryptographically
- [ ] File upload <5s for 100 MB file
- [ ] Web UI loads in <1.5s
- [ ] Cross-platform tested

---

## Phase 21+: Tier 3 Client Applications

**Priority:** LOW - Build based on user demand

### Phase 21: WRAITH-Stream (71 SP)

**Duration:** 8 weeks
**Focus:** Secure media streaming (video/audio)

**Reference:** [wraith-stream-sprints.md](clients/wraith-stream-sprints.md)

| Sprint | Focus | Points |
|--------|-------|--------|
| 21.1 | Streaming protocol, HLS/DASH-like | 8 |
| 21.2 | Video encoder (AV1/VP9), audio (Opus) | 42 |
| 21.3 | Adaptive bitrate, web player | 13 |
| 21.4 | Documentation, embed codes | 8 |

**Key Features:**
- Live and on-demand streaming
- AV1/VP9 video, Opus audio
- Adaptive bitrate
- <3 second latency (live)

---

### Phase 22: WRAITH-Mesh (60 SP)

**Duration:** 7 weeks
**Focus:** IoT mesh networking

**Reference:** [wraith-mesh-sprints.md](clients/wraith-mesh-sprints.md)

| Sprint | Focus | Points |
|--------|-------|--------|
| 22.1 | Mesh routing protocol (AODV-like) | 5 |
| 22.2 | Router daemon, multi-hop forwarding | 34 |
| 22.3 | Topology tests, failover | 13 |
| 22.4 | Deployment guide, compatibility matrix | 8 |

**Key Features:**
- Multi-hop routing
- Device pairing (QR codes)
- Route failover
- 100+ device scalability

---

### Phase 23: WRAITH-Publish (76 SP)

**Duration:** 8 weeks
**Focus:** Censorship-resistant publishing

**Reference:** [wraith-publish-sprints.md](clients/wraith-publish-sprints.md)

| Sprint | Focus | Points |
|--------|-------|--------|
| 23.1 | Content addressing (CID), DHT storage | 8 |
| 23.2 | Publisher GUI, reader, signatures | 47 |
| 23.3 | Propagation tests, censorship resistance | 13 |
| 23.4 | Publishing guide, legal considerations | 8 |

**Key Features:**
- IPFS-like content addressing
- DHT storage with replication
- Ed25519 content signatures
- <5 second publish latency

---

### Phase 24: WRAITH-Vault (94 SP)

**Duration:** 9 weeks
**Focus:** Distributed secret storage

**Reference:** [wraith-vault-sprints.md](clients/wraith-vault-sprints.md)

| Sprint | Focus | Points |
|--------|-------|--------|
| 24.1 | Shamir SSS, guardian selection | 13 |
| 24.2 | Shard encryption, recovery workflow | 55 |
| 24.3 | Recovery tests, security audit | 21 |
| 24.4 | User guide, disaster recovery | 5 |

**Key Features:**
- Shamir Secret Sharing (k-of-n)
- Guardian peer network
- Key rotation
- <10 second recovery

---

## Security Testing Clients

**Priority:** Specialized - requires governance framework

**Reference:** [ROADMAP-clients.md](ROADMAP-clients.md) (Tier 3: Security Testing)

### WRAITH-Recon (55 SP)

**Duration:** 12 weeks
**Focus:** Authorized reconnaissance and data transfer assessment

**Requirements:**
- Completed protocol Phase 7
- Signed Rules of Engagement (RoE)
- Kill switch capability
- Tamper-evident audit logging

### WRAITH-RedOps (89 SP)

**Duration:** 14 weeks
**Focus:** Comprehensive adversary emulation platform

**Requirements:**
- Completed WRAITH-Recon governance patterns
- Executive authorization
- Multi-operator audit trails
- MITRE ATT&CK mapping (51+ techniques)

**Note:** Security testing clients require [Security Testing Parameters](../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md) compliance.

---

## Document References

### Primary Planning Documents

| Document | Purpose | When to Consult |
|----------|---------|-----------------|
| [ROADMAP.md](ROADMAP.md) | Main project roadmap | Overall project direction |
| [ROADMAP-clients.md](ROADMAP-clients.md) | Client applications roadmap | Client development planning |
| [TECH-DEBT-v1.6.3.md](technical-debt/TECH-DEBT-v1.6.3.md) | Current technical debt | Before addressing debt items |

### Phase Completion Reports

| Document | Purpose | When to Consult |
|----------|---------|-----------------|
| [PHASE-17-COMPLETION-AUDIT.md](completed/PHASE-17-COMPLETION-AUDIT.md) | Phase 17 audit | Understanding current state |
| [to-dos/completed/](completed/) | Historical phase summaries | Patterns and lessons learned |

### Client Sprint Plans

| Document | Client | Story Points |
|----------|--------|--------------|
| [wraith-chat-sprints.md](clients/wraith-chat-sprints.md) | WRAITH-Chat | 182 SP |
| [wraith-sync-sprints.md](clients/wraith-sync-sprints.md) | WRAITH-Sync | 136 SP |
| [wraith-share-sprints.md](clients/wraith-share-sprints.md) | WRAITH-Share | 104 SP |
| [wraith-stream-sprints.md](clients/wraith-stream-sprints.md) | WRAITH-Stream | 71 SP |
| [wraith-mesh-sprints.md](clients/wraith-mesh-sprints.md) | WRAITH-Mesh | 60 SP |
| [wraith-publish-sprints.md](clients/wraith-publish-sprints.md) | WRAITH-Publish | 76 SP |
| [wraith-vault-sprints.md](clients/wraith-vault-sprints.md) | WRAITH-Vault | 94 SP |

### Technical Documentation

| Document | Purpose | When to Consult |
|----------|---------|-----------------|
| [UI-UX-DESIGN-REFERENCE.md](../docs/clients/UI-UX-DESIGN-REFERENCE.md) | UI/UX standards | Building client UIs |
| [docs/architecture/](../docs/architecture/) | System architecture | Understanding protocol design |
| [CLAUDE.md](../CLAUDE.md) | Project context | Starting any session |
| [CLAUDE.local.md](../CLAUDE.local.md) | Current state | Session-specific context |

### Protocol Phase Documents

| Document | Purpose |
|----------|---------|
| [PHASE-17-MASTER-STRATEGY.md](protocol/PHASE-17-MASTER-STRATEGY.md) | Mobile integration strategy |

---

## Success Criteria and Quality Gates

### Per-Phase Completion Criteria

#### Phase 18: Client Completion

- [ ] All 5 TD-014 TODO items resolved
- [ ] Voice/video edge case tests added
- [ ] Chat statistics enhanced
- [ ] Mobile device testing completed
- [ ] All 1,695+ tests passing
- [ ] Zero clippy warnings maintained

#### Phase 19: Infrastructure - COMPLETE (2026-01-24)

- [ ] DNS STUN resolution with fallback (TD-001 - remaining item)
- [x] ICE signaling per RFC 8445 (Sprint 19.2 - COMPLETE)
- [x] AF_XDP with UMEM configuration (Sprint 19.3 - COMPLETE)
- [x] Performance benchmarks documented
- [x] Backward compatibility maintained

#### Phase 20+: New Clients

- [ ] Tauri 2.0 + React frontend
- [ ] IPC command layer with type safety
- [ ] Protocol integration via wraith-core
- [ ] Test coverage >80%
- [ ] Cross-platform builds (Linux, macOS, Windows)
- [ ] Documentation complete

### Test Coverage Requirements

| Component | Minimum | Target |
|-----------|---------|--------|
| Protocol crates | 80% | 90% |
| Client backends | 70% | 80% |
| Client frontends | 60% | 70% |
| Integration tests | Required | Required |

### Documentation Requirements

- [ ] README.md updated with new features
- [ ] CHANGELOG.md entry for each release
- [ ] API documentation for new IPC commands
- [ ] User documentation for new clients
- [ ] Architecture diagrams for complex features

### Build/CI Requirements

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes (100%)
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo xtask ci` passes all checks
- [ ] Cross-platform builds succeed (Linux, macOS, Windows)

### Version Bump Checklist

- [ ] Update `Cargo.toml` workspace version
- [ ] Update `CLAUDE.md` metrics
- [ ] Update `CLAUDE.local.md` current state
- [ ] Update `CHANGELOG.md` with entry
- [ ] Tag release in git
- [ ] Generate release notes

---

## Appendix A: Technical Debt Reference

### Current Items (v1.6.3)

| ID | Issue | Severity | Priority |
|----|-------|----------|----------|
| TH-006 | AF_XDP Socket Implementation | ~~HIGH~~ | ✅ COMPLETE |
| TM-001 | Full ICE Signaling | ~~HIGH~~ | ✅ COMPLETE |
| TD-001 | DNS STUN Resolution | MEDIUM | P3 |
| TD-014 | Chat Minor TODOs (5 items) | MEDIUM | P3 |
| TL-001 | Pedantic Warnings (~800) | LOW | P5 |
| TL-002 | Additional Test Coverage | LOW | P4 |
| TL-003 | Chat Statistics Enhancement | LOW | P5 |
| TL-004 | Mobile Device Testing | LOW | P3 |

### Resolved in Phase 17

| ID | Issue | Resolution |
|----|-------|------------|
| TD-002 | Android transfer tracking | Full transfer state management |
| TD-003 | Android unwrap() cleanup | Result-based error handling |
| TD-004 | iOS file size | Actual file metadata query |
| TD-005 | iOS transfer tracking | Full transfer state management |
| TD-006 | iOS unwrap() cleanup | UniFFI error types |
| TD-007 | WRAITH node integration | Full protocol integration |
| TD-008 | Database key handling | Platform keyring integration |
| TD-009 | Double Ratchet init | X25519 key exchange |
| TD-010 | Message sending | WRAITH protocol streams |
| TD-011 | Node initialization | Real peer ID from node |
| TD-012 | Statistics | Basic stats implemented |
| TD-013 | Crypto unwraps | Proper error handling |

---

## Appendix B: Quick Command Reference

### Development Commands

```bash
# Build and test
cargo build --workspace           # Build all crates
cargo test --workspace            # Run all tests
cargo clippy --workspace -- -D warnings  # Lint check
cargo fmt --all                   # Format code
cargo xtask ci                    # All CI checks

# Client development
cd clients/wraith-chat && npm run tauri dev     # Chat dev
cd clients/wraith-transfer && npm run tauri dev # Transfer dev
cd clients/wraith-sync && npm run tauri:dev     # Sync dev

# Documentation
cargo doc --workspace --open      # Generate docs
```

### Git Workflow

```bash
# Feature branch
git checkout -b feature/phase-18-chat-wireup

# Commit with conventional format
git commit -m "feat(chat): wire up group messaging via WRAITH protocol"

# PR creation
gh pr create --title "Phase 18: WRAITH-Chat Protocol Wire-Up"
```

---

**Document Version:** 1.0.0
**Created:** 2026-01-21
**Status:** Authoritative Development Guide

**Next Review:** After Phase 18 completion
**Archive:** `to-dos/completed/` after each phase

---

*This document serves as the authoritative guide for all remaining WRAITH Protocol development work. Consult referenced documents for detailed sprint plans and implementation specifications.*
