# WRAITH Protocol - Client Applications Development History

**Development Timeline:** Phase 15-16 (2025-12-11) - 4 Client Applications Complete

This document tracks the development journey of WRAITH Protocol client applications, from planning through implementation and release. Phase 15 delivered WRAITH-Transfer desktop application, and Phase 16 delivered Android/iOS mobile clients plus WRAITH-Chat E2EE messaging.

[![Version](https://img.shields.io/badge/clients-4%20complete-green.svg)](https://github.com/doublegate/WRAITH-Protocol/releases)
[![Protocol](https://img.shields.io/badge/protocol-v1.6.2-blue.svg)](../../README.md)
[![Clients](https://img.shields.io/badge/clients-6%20planned-orange.svg)](../../to-dos/ROADMAP-clients.md)

---

## Overview

WRAITH Protocol's client ecosystem encompasses **10 specialized applications** across **3 priority tiers**, providing comprehensive secure communication, file transfer, and collaboration capabilities. All clients share the same cryptographic foundation while offering specialized features for different use cases.

For the main project README, see [../../README.md](../../README.md).
For protocol development history, see [README_Protocol-DEV.md](README_Protocol-DEV.md).

---

## Client Ecosystem Summary

**Total Development Scope:**
- **10 Client Applications** (8 standard + 2 security testing)
- **1,148 Story Points** total (404 SP delivered + 744 SP remaining)
- **~40 weeks remaining** (parallel development across tiers)
- **26 development phases** across all clients

**Development Strategy:**
- **Tier 1:** High-priority core applications (Transfer ✅, Android ✅, iOS ✅, Chat ✅ Complete)
- **Tier 2:** Specialized productivity tools (Sync, Share)
- **Tier 3:** Advanced use cases (Stream, Mesh, Publish, Vault)
- **Security Testing:** Authorized assessment tools (Recon, RedOps)

**Current Status (2026-01-21):**
- Protocol v1.6.2 complete (all prerequisites available)
- **Phase 15 - WRAITH-Transfer v1.6.2:** ✅ **COMPLETE** (102 SP)
  - Cross-platform desktop P2P file transfer
  - Tauri 2.0 + React 18 + TypeScript
  - 62 frontend tests with Vitest (100% pass rate)
- **Phase 16 - Mobile & Chat v1.6.2:** ✅ **COMPLETE** (302 SP)
  - **WRAITH-Android:** Native Kotlin + Jetpack Compose (~60 SP)
  - **WRAITH-iOS:** Native Swift + SwiftUI (~60 SP) - proper error handling
  - **WRAITH-Chat:** E2EE messaging with Double Ratchet (182 SP)
    - Full WRAITH protocol integration (TD-007 to TD-011)
    - Secure key storage with platform-native keyring
    - Real peer identity from node.node_id()
- **Development Status:** 4 of 10 Tier 1 clients complete (404 SP delivered, 744 SP remaining)
- **CI/CD:** GitHub Actions updated (upload-artifact v6, download-artifact v7, cache v5)
- **Tech Debt:** TD-006 (iOS UniFFI unwrap safety) remediated

---

## Client Applications Overview

### Tier 1: Core Applications (High Priority - 404 SP)

| # | Client | Description | Platform | Story Points | Status |
|---|--------|-------------|----------|--------------|--------|
| 1 | **WRAITH-Transfer** | Direct P2P file transfer with drag-and-drop GUI | Desktop (Linux/macOS/Windows) | 102 | ✅ **Complete (v1.6.0)** |
| 2 | **WRAITH-Android** | Native Android mobile file transfer client | Android 8.0+ | ~60 | ✅ **Complete (v1.6.0)** |
| 3 | **WRAITH-iOS** | Native iOS mobile file transfer client | iOS 16.0+ | ~60 | ✅ **Complete (v1.6.0)** |
| 4 | **WRAITH-Chat** | E2EE messaging with Signal Protocol Double Ratchet | Desktop | 182 | ✅ **Complete (v1.6.0)** |

**WRAITH Transfer Delivered (2025-12-11):**
- Tauri 2.0 desktop application with full wraith-core integration
- React 18 + TypeScript frontend with Vite bundling
- Tailwind CSS v4 with WRAITH brand colors (#FF5722, #4A148C)
- 10 IPC commands for node/session/transfer management
- 5 React components with real-time status updates
- 3 Zustand stores for state management
- Cross-platform builds for Windows, macOS, Linux (X11/Wayland)
- FFI layer (wraith-ffi crate) for C-compatible API
- v1.5.8: Wayland compatibility fix (resolves KDE Plasma 6 crashes)
- v1.5.9: Tauri 2.0 capability-based permissions (dialog, fs, shell plugins working)

**WRAITH Chat Timeline:** Planned Q2-Q3 2026 (12 weeks development)
**Prerequisites:** Protocol Phase 6 (Integration) - ✅ Complete

---

### Tier 2: Specialized Applications (Medium Priority - 259 SP)

| # | Client | Description | Platform | Story Points | Status |
|---|--------|-------------|----------|--------------|--------|
| 3 | **WRAITH-Sync** | Decentralized backup synchronization (Dropbox alternative) | Desktop + CLI | 136 | Planned |
| 4 | **WRAITH-Share** | Distributed anonymous file sharing (BitTorrent-like) | Desktop + Web UI | 123 | Planned |

**Timeline:** Planned Q2-Q3 2026 (20 weeks parallel development)
**Prerequisites:** Protocol Phase 5 (Discovery) - ✅ Complete

---

### Tier 3: Advanced Applications (Lower Priority - 361 SP)

| # | Client | Description | Platform | Story Points | Status |
|---|--------|-------------|----------|--------------|--------|
| 5 | **WRAITH-Stream** | Secure media streaming (live/VOD with AV1/Opus) | Desktop + Web | 71 | Planned |
| 6 | **WRAITH-Mesh** | IoT mesh networking for device communication | Embedded Linux + Desktop | 60 | Planned |
| 7 | **WRAITH-Publish** | Censorship-resistant publishing (blogs, wikis) | Desktop + Web | 76 | Planned |
| 8 | **WRAITH-Vault** | Distributed secret storage (Shamir Secret Sharing) | Desktop + CLI | 94 | Planned |

**Timeline:** Planned Q3-Q4 2026 (20 weeks batched development)
**Prerequisites:** Protocol Phase 6 (Integration) - ✅ Complete

---

### Tier 3: Security Testing (Specialized - 144 SP)

| # | Client | Description | Platform | Story Points | Status |
|---|--------|-------------|----------|--------------|--------|
| 9 | **WRAITH-Recon** | Network reconnaissance & data exfiltration assessment | Linux (kernel 6.2+) | 55 | Planned |
| 10 | **WRAITH-RedOps** | Red team operations platform with C2 infrastructure | Team Server + Operator Client + Implant | 89 | Planned |

**Timeline:** Planned Q3 2026+ (26 weeks sequential development)
**Prerequisites:** Protocol Phase 7 (Hardening) - ✅ Complete

**⚠️ GOVERNANCE NOTICE:** Security testing clients require signed authorization, scope enforcement, audit logging, and compliance with [Security Testing Parameters](../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md).

---

## Development Timeline (Planned)

### Phase 15: WRAITH Transfer Desktop Application - ✅ COMPLETE (2025-12-09)

**Completion Date:** 2025-12-09
**Story Points Delivered:** 102 SP (100% complete)

**Focus:** Production-ready cross-platform desktop application with Tauri 2.0 backend and React 18 frontend

#### Sprint 15.1: FFI Core Library Bindings (21 SP) - ✅ COMPLETE
- ✅ **wraith-ffi crate** - C-compatible API for language interoperability
  - FFI-safe types with #[repr(C)] for ABI stability
  - Node lifecycle functions (wraith_node_new, wraith_node_start, wraith_node_stop, wraith_node_free)
  - Session management (wraith_establish_session, wraith_close_session)
  - File transfer functions (wraith_send_file, wraith_get_transfer_progress)
  - Error handling with FFI-safe error codes and messages
  - Memory safety guarantees with proper ownership transfer
  - 7 comprehensive tests validating FFI boundary safety
- ✅ **C header generation** - cbindgen integration for automatic header file generation

#### Sprint 15.2: Tauri Desktop Shell (34 SP) - ✅ COMPLETE
- ✅ **Tauri 2.0 Backend** (`clients/wraith-transfer/src-tauri/`)
  - lib.rs (84 lines) - Main entry point with IPC handler registration
  - commands.rs (315 lines) - 10 IPC commands for protocol control
  - state.rs - AppState with Arc<RwLock<Option<Node>>> for thread-safe state
  - error.rs - AppError enum with Serialize for frontend communication
  - Cargo.toml - Tauri 2.9.4 with plugins (dialog, fs, shell, log)
- ✅ **IPC Command Reference:**
  - start_node(), stop_node(), get_node_status()
  - establish_session(peer_id), close_session(peer_id)
  - send_file(peer_id, file_path), cancel_transfer(transfer_id)
  - get_transfers(), get_sessions(), get_logs(level)
- ✅ **Tauri Plugins:** dialog, fs, shell, log integration
- ✅ **Thread Safety:** Arc<RwLock<Option<Node>>> for shared mutable state

#### Sprint 15.3: React UI Foundation (23 SP) - ✅ COMPLETE
- ✅ **React 18 + TypeScript Frontend** (`clients/wraith-transfer/frontend/`)
  - Vite 7.2.7 build system with Hot Module Replacement (HMR)
  - Tailwind CSS v4 with WRAITH brand colors (#FF5722 primary, #4A148C secondary)
  - TypeScript strict mode for type safety
- ✅ **Type Definitions** (lib/types.ts)
  - NodeStatus, TransferInfo, SessionInfo interfaces
- ✅ **State Management** (Zustand stores)
  - nodeStore.ts, transferStore.ts, sessionStore.ts
- ✅ **Tauri IPC Bindings** (lib/tauri.ts)
  - Full TypeScript bindings for all 10 backend commands
  - Type-safe invoke wrappers with error handling

#### Sprint 15.4: Transfer UI Components (24 SP) - ✅ COMPLETE
- ✅ **Core Components** (`src/components/`)
  - Header.tsx - Connection status, node ID, session/transfer counts, start/stop button
  - TransferList.tsx - Transfer items with progress bars, speed/ETA, cancel buttons
  - SessionPanel.tsx - Active sessions sidebar with disconnect capability
  - NewTransferDialog.tsx - Modal for initiating transfers with file picker
  - StatusBar.tsx - Quick actions, error display, "New Transfer" button
- ✅ **Main Application** (App.tsx)
  - Full layout with header, main content, sidebar, status bar
  - 1-second polling for status updates when node is running
  - Dialog state management for transfer initiation

**Phase 15 Deliverables - ALL COMPLETE:**
- ✅ Production-ready desktop application for Windows, macOS, Linux
- ✅ Cross-platform builds with Tauri 2.0
- ✅ Full file transfer operations via intuitive GUI
- ✅ Real-time status monitoring and progress tracking
- ✅ FFI layer (wraith-ffi) for future language bindings
- ✅ 1,679+ total tests (1,617 Rust + 62 frontend Vitest tests)
- ✅ Zero clippy warnings, zero TypeScript errors
- ✅ CI/CD pipeline with Tauri system dependencies
- ✅ Frontend test infrastructure with Testing Library

---

### v1.6.2: Protocol Integration & Infrastructure (2026-01-21)

**Focus:** Complete WRAITH protocol integration for WRAITH-Chat client

**Key Accomplishments:**

**WRAITH-Chat Protocol Integration (TD-007 to TD-011):**
- **WraithNode Integration:** Full Node wrapper in chat state with lifecycle management
  - start_node() properly initializes WRAITH protocol
  - Real peer identity from node.node_id()
  - Session management for peer connections
- **Secure Key Storage:** Platform-native keyring integration
  - Linux: libsecret (D-Bus Secret Service)
  - macOS: Keychain
  - Windows: Credential Manager
  - Secure storage of identity keys and ratchet state
- **Double Ratchet Key Exchange:** Integrated with X25519 from WRAITH protocol
  - Keys derived from WRAITH crypto primitives
  - Session establishment uses WRAITH handshake
- **Message Transmission:** Via WRAITH protocol streams with encryption
  - Messages sent over encrypted WRAITH sessions
  - Full traffic obfuscation applied
- **Chat Tests Updated:** 6 passing tests for backend functionality

**Mobile Client Verification:**
- **iOS UniFFI Safety (TD-006):** All unwrap() calls replaced with proper Result error handling
- **Android JNI (TD-002 to TD-005):** Confirmed implemented in Phase 16
- **Error Handling (TD-012, TD-013):** Proper error propagation across FFI boundary

**Quality Metrics:**
- All client tests passing
- Zero TypeScript errors in frontends
- Tauri 2.0 capability-based permissions working
- Cross-platform builds verified

---

### v1.5.9: Tauri 2.0 Configuration Fix & Protocol Enhancements (2025-12-11)

**Focus:** Tauri 2.0 capability-based permissions, protocol enhancements for client applications

**Key Accomplishments:**

**WRAITH Transfer Tauri 2.0 Fix:**
- **Capability-Based Permissions:** Updated to Tauri 2.0 permission model
  - Migrated plugin permissions from tauri.conf.json to capabilities/default.json
  - Added dialog:default, dialog:allow-open, dialog:allow-save, dialog:allow-message
  - Added fs:default, fs:allow-read-dir, fs:allow-read-file, fs:allow-write-file
  - Added shell:default, shell:allow-open
- **Plugin Initialization Error Resolved:**
  - Fixed `PluginInitialization("dialog", "invalid type: map, expected unit")` panic
  - All Tauri plugins (dialog, fs, shell, log) now functioning correctly
- **Files Updated:** `clients/wraith-transfer/src-tauri/capabilities/default.json`, `tauri.conf.json`

**Protocol Enhancements for Clients:**
- **New CLI Commands Available for Integration:**
  - `wraith ping` - Network connectivity testing (RTT statistics)
  - `wraith config show/set` - Runtime configuration management
- **Multi-Peer Transfer Support:**
  - Protocol now supports parallel transfers to multiple recipients
  - Enhanced for future multi-user client applications
- **NAT Detection Reliability:**
  - 5 STUN servers from 4 providers for improved connectivity
  - Graceful degradation on individual server failures
- **Enhanced Node API:**
  - Improved session management for client applications
  - Better error handling across FFI boundary

**Quality Assurance:**
- WRAITH Transfer: Production-ready with fixed Tauri 2.0 integration
- Protocol: 1,613 total tests (+217 from v1.5.8)
- Zero clippy warnings, zero TypeScript errors
- Cross-platform compatibility verified

---

### Phase 16: Mobile Clients (Planned Q2-Q4 2026)

**Target Completion:** Q4 2026
**Estimated Story Points:** ~120 SP

**Focus:** Android and iOS applications for Transfer and Chat

#### Sprint 16.1-16.2: Android Client
- [ ] Kotlin/Rust interop via JNI (native library integration)
- [ ] Jetpack Compose UI (Material Design 3)
- [ ] Background service (foreground service for transfers)
- [ ] Notification integration (progress, completion, errors)

#### Sprint 16.3-16.4: iOS Client
- [ ] Swift/Rust interop via UniFFI (automated bindings generation)
- [ ] SwiftUI interface (native iOS design patterns)
- [ ] Background task handling (URLSession background uploads)
- [ ] Share extension (send files from other apps)

**Phase 16 Deliverables:**
- Android app (Play Store ready, API 26+)
- iOS app (App Store ready, iOS 15+)
- Mobile-optimized UI/UX

---

### Phase 17: SDKs and Libraries (Planned Q1 2027)

**Target Completion:** Q1 2027
**Estimated Story Points:** ~100 SP

**Focus:** Language bindings for developer integration

#### Sprint 17.1: Python SDK
- [ ] PyO3 bindings (Rust ↔ Python FFI)
- [ ] Async support (asyncio integration, async/await)
- [ ] Type hints (complete .pyi stub files)
- [ ] PyPI package (wheels for Linux/macOS/Windows)

#### Sprint 17.2: Go SDK
- [ ] CGO bindings (Rust static library → Go)
- [ ] Go-native error handling (error interface)
- [ ] Context support (cancellation, timeouts)
- [ ] Module publishing (go.mod, versioned releases)

#### Sprint 17.3: Node.js SDK
- [ ] N-API bindings (native addon with Rust backend)
- [ ] Promise-based API (async Node.js patterns)
- [ ] TypeScript definitions (.d.ts for autocomplete)
- [ ] npm package (native modules for all platforms)

#### Sprint 17.4: C Library
- [ ] Pure C API (stable ABI, no C++ dependencies)
- [ ] Header generation (automatic from Rust with cbindgen)
- [ ] Static/dynamic linking options
- [ ] pkg-config support (Linux standard integration)

**Phase 17 Deliverables:**
- Language SDKs with full API coverage
- Package manager distribution (PyPI, npm, crates.io)
- Comprehensive API documentation

---

### Phase 18: Web and Embedded (Planned Q2 2027)

**Target Completion:** Q2 2027
**Estimated Story Points:** ~80 SP

**Focus:** Browser-based and embedded deployments

#### Sprint 18.1-18.2: Web Client
- [ ] WebAssembly compilation (wasm32-unknown-unknown target)
- [ ] WebRTC transport adaptation (TURN/STUN for NAT)
- [ ] Progressive Web App (service workers, offline support)
- [ ] Browser extension (WebExtension API for all browsers)

#### Sprint 18.3-18.4: Embedded Client
- [ ] no_std Rust implementation (zero std library dependencies)
- [ ] Minimal memory footprint (<1 MB RAM for basic operations)
- [ ] RTOS integration examples (FreeRTOS, Zephyr)
- [ ] Hardware crypto support (AES-NI, ARM TrustZone)

**Phase 18 Deliverables:**
- Browser-based file transfer (WASM + WebRTC)
- Embedded device support (IoT integration)
- Reference implementations for common platforms

---

## Development Metrics (Planned)

### Story Points by Phase

| Phase | Focus | Target SP | Actual SP | Status |
|-------|-------|-----------|-----------|--------|
| Phase 15 | Reference Client (Transfer) | 102 | 102 | ✅ **Complete** |
| Phase 16 | Mobile Clients | ~120 | - | Planned |
| Phase 17 | SDKs & Libraries | ~100 | - | Planned |
| Phase 18 | Web & Embedded | ~80 | - | Planned |
| **Total** | **Client Foundation** | **~402** | **102** | **25% Complete** |

### Client Implementation Status

| Client | Spec | Design | Core | UI | Tests | Docs | Release |
|--------|------|--------|------|----|----|------|---------|
| Transfer | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ v1.6.0 |
| Chat | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ v1.6.0 |
| Android | ✅ | ✅ | ✅ | ✅ | ⬜ | ✅ | ✅ v1.6.0 |
| iOS | ✅ | ✅ | ✅ | ✅ | ⬜ | ✅ | ✅ v1.6.0 |
| Sync | ✅ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ |
| Share | ✅ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ |
| Stream | ✅ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ |
| Mesh | ✅ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ |
| Publish | ✅ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ |
| Vault | ✅ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ |
| Recon | ✅ | ⬜ | ⬜ | N/A | ⬜ | ⬜ | ⬜ |
| RedOps | ✅ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ |

**Legend:**
- ✅ Complete
- 🔄 In Progress
- ⬜ Not Started
- N/A Not Applicable

---

## Quality Milestones (Planned)

### Test Coverage Goals

| Client Category | Unit Tests | Integration | E2E | Target Coverage |
|-----------------|------------|-------------|-----|-----------------|
| Desktop Clients | - | - | - | 80% |
| Mobile Clients | - | - | - | 75% |
| SDKs | - | - | - | 90% |
| Web/WASM | - | - | - | 80% |

### Performance Targets

| Metric | Desktop | Mobile | Web | SDK |
|--------|---------|--------|-----|-----|
| Cold Start | <2s | <3s | <5s | N/A |
| Transfer Init | <100ms | <200ms | <500ms | <50ms |
| Memory (Idle) | <50MB | <30MB | <20MB | <10MB |
| Binary Size | <20MB | <15MB | <5MB WASM | <2MB |

**Notes:**
- Desktop: Tauri applications (Rust backend + webview)
- Mobile: Native apps (Kotlin/Swift with Rust core)
- Web: WASM + WebRTC transport
- SDK: Language bindings with minimal overhead

---

## Technical Architecture Decisions

### Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Desktop Framework | Tauri 2.0 | Rust backend, small binary (<20MB), cross-platform, native webview |
| UI Framework | React 18 + TypeScript | Mature ecosystem, component reusability, type safety |
| State Management | Zustand | Lightweight (<1KB), TypeScript-native, minimal boilerplate |
| Styling | Tailwind CSS + shadcn/ui | Utility-first, consistent design system, accessibility built-in |
| Mobile Android | Kotlin + Jetpack Compose | Modern Android development, declarative UI |
| Mobile iOS | Swift + SwiftUI | Native iOS, declarative UI, performance |
| Mobile Interop | UniFFI | Automated Swift/Kotlin bindings from Rust |
| WASM Target | wasm32-unknown-unknown | Standard target, broad browser support |

### Platform Support Matrix

| Platform | Minimum Version | Architecture | Priority |
|----------|-----------------|--------------|----------|
| **Desktop** | | | |
| Windows | 10 (1903+) | x86_64, arm64 | Tier 1 |
| macOS | 11.0+ (Big Sur) | x86_64, arm64 | Tier 1 |
| Linux | glibc 2.31+ | x86_64, arm64 | Tier 1 |
| **Mobile** | | | |
| Android | API 26+ (8.0 Oreo) | arm64-v8a, armeabi-v7a | Tier 2 |
| iOS | 15.0+ | arm64 | Tier 2 |
| **Web** | | | |
| Browsers | ES2020+ | wasm32 | Tier 2 |

**Tier 1:** Full support, CI testing, priority bug fixes
**Tier 2:** Best-effort support, community-driven testing

---

## Client-Specific Specifications

### Tier 1: Core Applications

#### WRAITH-Transfer (102 SP, 13 weeks)
**Purpose:** Direct P2P file transfer with drag-and-drop GUI

**Documentation:**
- [Architecture](../clients/wraith-transfer/architecture.md) - Technical design, protocol integration
- [Features](../clients/wraith-transfer/features.md) - File transfer capabilities, multi-peer support
- [Implementation](../clients/wraith-transfer/implementation.md) - Code structure, API reference

**Key Features:**
- Drag-and-drop file selection
- Multi-peer parallel downloads
- Resume/seek functionality
- BLAKE3 integrity verification
- Progress tracking (speed, ETA, percentage)

---

#### WRAITH-Chat (162 SP, 13 weeks)
**Purpose:** E2EE messaging with Double Ratchet algorithm

**Documentation:**
- [Architecture](../clients/wraith-chat/architecture.md) - Message protocol, ratchet state machine
- [Features](../clients/wraith-chat/features.md) - 1-on-1, group chat, voice/video
- [Implementation](../clients/wraith-chat/implementation.md) - Message database, UI components

**Key Features:**
- 1-on-1 and group encrypted messaging
- File attachments (via wraith-files)
- Voice calling (Opus codec)
- Video calling (AV1 codec)
- Message persistence (encrypted SQLite)

---

### Tier 2: Specialized Applications

#### WRAITH-Sync (136 SP, 13 weeks)
**Purpose:** Decentralized backup and file synchronization

**Documentation:**
- [Architecture](../clients/wraith-sync/architecture.md) - Sync protocol, conflict resolution
- [Features](../clients/wraith-sync/features.md) - Delta sync, multi-device orchestration
- [Implementation](../clients/wraith-sync/implementation.md) - File system watcher, change detection

**Key Features:**
- Real-time file watching (inotify, FSEvents, ReadDirectoryChangesW)
- Delta sync algorithm (rsync-like)
- Conflict resolution (3-way merge)
- Selective sync (folder inclusion/exclusion)
- Bandwidth throttling

---

#### WRAITH-Share (123 SP, 12 weeks)
**Purpose:** Distributed anonymous file sharing (BitTorrent-like)

**Documentation:**
- [Architecture](../clients/wraith-share/architecture.md) - Swarm protocol, piece selection
- [Features](../clients/wraith-share/features.md) - DHT content addressing, multi-peer downloads
- [Implementation](../clients/wraith-share/implementation.md) - Swarm manager, piece downloader

**Key Features:**
- DHT content addressing (announce, lookup)
- Swarm downloads (parallel chunk fetching)
- Piece selection strategy (rarest-first)
- Magnet link support
- Web seed integration

---

### Tier 3: Advanced Applications

#### WRAITH-Stream (71 SP, 8 weeks)
**Purpose:** Secure media streaming (live/VOD)

**Documentation:**
- [Architecture](../clients/wraith-stream/architecture.md) - Streaming protocol, adaptive bitrate
- [Features](../clients/wraith-stream/features.md) - Live/VOD streaming, codec support
- [Implementation](../clients/wraith-stream/implementation.md) - Encoder pipeline, player

**Key Features:**
- Video encoding (AV1/VP9)
- Audio encoding (Opus)
- Adaptive bitrate logic
- Live streaming support
- Web player (video.js)

---

#### WRAITH-Mesh (60 SP, 7 weeks)
**Purpose:** IoT mesh networking for device communication

**Documentation:**
- [Architecture](../clients/wraith-mesh/architecture.md) - Mesh routing protocol, topology discovery
- [Features](../clients/wraith-mesh/features.md) - Multi-hop routing, device pairing
- [Implementation](../clients/wraith-mesh/implementation.md) - Router daemon, configuration API

**Key Features:**
- Mesh routing protocol (AODV-like)
- Route discovery and multi-hop forwarding
- Device pairing (QR codes)
- Web-based configurator
- Network visualization

---

#### WRAITH-Publish (76 SP, 8 weeks)
**Purpose:** Censorship-resistant publishing platform

**Documentation:**
- [Architecture](../clients/wraith-publish/architecture.md) - Content addressing, DHT storage
- [Features](../clients/wraith-publish/features.md) - Publishing protocol, content signatures
- [Implementation](../clients/wraith-publish/implementation.md) - Publisher GUI, reader

**Key Features:**
- Content chunking & addressing (IPFS-like CIDs)
- DHT storage (announce, retrieve)
- Publisher GUI (Markdown editor)
- Web-based reader
- Content signatures (Ed25519)

---

#### WRAITH-Vault (94 SP, 9 weeks)
**Purpose:** Distributed secret storage (Shamir Secret Sharing)

**Documentation:**
- [Architecture](../clients/wraith-vault/architecture.md) - Shamir SSS, guardian peer selection
- [Features](../clients/wraith-vault/features.md) - Shard distribution, recovery protocol
- [Implementation](../clients/wraith-vault/implementation.md) - SSS implementation, CLI/GUI

**Key Features:**
- Shamir Secret Sharing (k-of-n configuration)
- Shard encryption
- Guardian peer management
- Recovery workflow
- CLI and desktop GUI

---

### Tier 3: Security Testing (Authorized Use Only)

#### WRAITH-Recon (55 SP, 12 weeks)
**Purpose:** Network reconnaissance & data exfiltration assessment

**Classification:** Security Testing Tool - Requires Authorization

**Documentation:**
- [Architecture](../clients/wraith-recon/architecture.md) - Technical design, protocol integration
- [Features](../clients/wraith-recon/features.md) - Reconnaissance, exfiltration capabilities
- [Implementation](../clients/wraith-recon/implementation.md) - Reference implementation
- [Integration](../clients/wraith-recon/integration.md) - Tool compatibility, MITRE ATT&CK
- [Testing](../clients/wraith-recon/testing.md) - Protocol verification, evasion testing
- [Usage](../clients/wraith-recon/usage.md) - Operator workflows, configuration

**Key Features:**
- AF_XDP wire-speed reconnaissance (10-40 Gbps)
- Protocol mimicry (TLS 1.3, DoH, WebSocket, ICMP)
- Multi-path exfiltration (UDP/TCP/HTTPS/DNS/ICMP)
- Passive & active scanning
- Governance controls (target whitelist, time bounds, audit logging)

---

#### WRAITH-RedOps (89 SP, 14 weeks)
**Purpose:** Red team operations platform with C2 infrastructure

**Classification:** Security Testing Tool - Requires Executive Authorization

**Documentation:**
- [Architecture](../clients/wraith-redops/architecture.md) - C2 infrastructure, implant design
- [Features](../clients/wraith-redops/features.md) - Post-exploitation capabilities
- [Implementation](../clients/wraith-redops/implementation.md) - Team server, beacon, operator client
- [Integration](../clients/wraith-redops/integration.md) - Protocol stack, tool compatibility
- [Testing](../clients/wraith-redops/testing.md) - Cryptographic verification, evasion testing
- [Usage](../clients/wraith-redops/usage.md) - Operator workflows, protocol configuration

**Key Features:**
- Team Server (multi-user, PostgreSQL, gRPC API)
- Operator Console (Tauri GUI, session management)
- Spectre Implant (no_std Rust, PIC, sleep mask, indirect syscalls)
- Multi-transport C2 (UDP, TCP, HTTPS, DNS, WebSocket)
- P2P beacon mesh (SMB, TCP lateral movement)
- MITRE ATT&CK coverage (51+ techniques across 12 tactics)

**⚠️ GOVERNANCE:** Requires signed RoE, executive authorization, audit logging, kill switch mechanisms. See [Security Testing Parameters](../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md).

---

## Development Dependencies

### Shared Components (Cross-Client)

**Component:** Contact/Peer Management
- **Used By:** Chat, Share, Publish, Vault
- **Crate:** `wraith-contacts` (to be created in Phase 15)
- **Development:** Before Tier 1 client work begins

**Component:** File Transfer Engine
- **Used By:** Transfer, Sync, Share, Chat (attachments), Recon, RedOps
- **Crate:** `wraith-files` (protocol Phase 6) - ✅ Complete
- **Status:** Ready for integration

**Component:** DHT Client
- **Used By:** All clients (peer discovery)
- **Crate:** `wraith-discovery` (protocol Phase 5) - ✅ Complete
- **Status:** Ready for integration

**Component:** GUI Framework (Tauri)
- **Used By:** Transfer, Chat, Sync, Share, Stream, RedOps (Operator Client)
- **Shared Library:** `wraith-gui-common` (to be created in Phase 15)
- **Development:** Sprint 15.2-15.3

---

## Development Order Rationale

1. **Transfer First:** Simplest client, validates file transfer engine integration
2. **Chat Second:** Validates ratcheting, builds on Transfer's peer management
3. **Sync Third:** Builds on Transfer's file engine, adds delta sync complexity
4. **Share Fourth:** Builds on Transfer + DHT, validates swarm logic
5. **Stream Fifth:** Builds on Transfer's streaming, adds codec integration
6. **Mesh Sixth:** Validates multi-hop routing (unique networking challenge)
7. **Publish Seventh:** Builds on Share's DHT storage patterns
8. **Vault Eighth:** Builds on DHT, adds Shamir Secret Sharing
9. **Recon Ninth:** Requires completed protocol, validates obfuscation effectiveness
10. **RedOps Tenth:** Builds on Recon governance, most complex client (multi-component)

---

## Integration Timeline (Gantt Overview)

### Protocol Development (Complete)
```
Weeks  1---5---10---15---20---25---30---35---40---45---50---55---60---65---70
Phase 1-12 [==============================================] ✅ COMPLETE
```

### Tier 1 Clients (Q1-Q2 2026)
```
Weeks  1---5---10---15---20---25---30---35---40---45---50---55---60---65---70
Transfer                    [============]
Chat                        [============]
```

### Tier 2 Clients (Q2-Q3 2026)
```
Weeks  1---5---10---15---20---25---30---35---40---45---50---55---60---65---70
Sync                                [============]
Share                               [===========]
```

### Tier 3 Clients (Q3-Q4 2026)
```
Weeks  1---5---10---15---20---25---30---35---40---45---50---55---60---65---70
Stream                                          [=======]
Mesh                                            [======]
Publish                                         [=======]
Vault                                           [========]
```

### Security Testing Clients (Q3 2026+)
```
Weeks  1---5---10---15---20---25---30---35---40---45---50---55---60---65---70
Recon                                               [===========]
RedOps                                                          [=============]
```

---

## Story Points Summary

### By Tier

| Tier | Clients | Story Points | Duration | Developers |
|------|---------|--------------|----------|------------|
| **Tier 1** | Transfer, Chat | 264 | 16 weeks | 2-3 (parallel) |
| **Tier 2** | Sync, Share | 259 | 20 weeks | 2 (parallel) |
| **Tier 3** | Stream, Mesh, Publish, Vault | 361 | 20 weeks | 1-2 (batched) |
| **Security** | Recon, RedOps | 144 | 26 weeks | 1-2 (sequential) |
| **Total** | **10 clients** | **1,028** | **70 weeks** | **2-3 average** |

### By Client (Detailed)

| Client | Story Points | Duration | Prerequisites | Status |
|--------|--------------|----------|---------------|--------|
| Transfer | 102 | 13 weeks | Protocol Phase 6 ✅ | Planned |
| Chat | 162 | 13 weeks | Protocol Phase 5 ✅ | Planned |
| Sync | 136 | 13 weeks | Protocol Phase 6 ✅ | Planned |
| Share | 123 | 12 weeks | Protocol Phase 5 ✅ | Planned |
| Stream | 71 | 8 weeks | Protocol Phase 6 ✅ | Planned |
| Mesh | 60 | 7 weeks | Protocol Phase 5 ✅ | Planned |
| Publish | 76 | 8 weeks | Protocol Phase 5 ✅ | Planned |
| Vault | 94 | 9 weeks | Protocol Phase 6 ✅ | Planned |
| Recon | 55 | 12 weeks | Protocol Phase 7 ✅ + Governance | Planned |
| RedOps | 89 | 14 weeks | Protocol Phase 7 ✅ + Recon Governance | Planned |

**Note:** All protocol prerequisites are complete (v1.5.5 released 2025-12-08).

---

## Current Status & Next Steps

**Protocol Status (2026-01-21):**
- ✅ All 16 protocol development phases complete (1,937 SP delivered)
- ✅ 1,700+ tests passing (1,630 Rust + 62 frontend) - 100% pass rate
- ✅ Zero vulnerabilities, zero clippy warnings
- ✅ Grade A+ quality (98/100)
- ✅ Production-ready architecture with v1.6.2 release
- ✅ Full WRAITH protocol integration in WRAITH-Chat
- ✅ Secure key storage with platform-native keyring
- ✅ AF_XDP socket configuration for kernel bypass
- ✅ ICE signaling with RFC 8445 connectivity checks
- ✅ DNS-based STUN resolution with caching
- ✅ CI/CD updated (GitHub Actions: upload-artifact v6, download-artifact v7, cache v5)

**Client Development Status:**
- ✅ Comprehensive planning complete (roadmap, specifications)
- ✅ All client specifications documented (10 clients × 3-6 docs each)
- ✅ **All 4 Tier 1 clients complete** (404 SP delivered, Phases 15-16)
  - WRAITH-Transfer: Desktop P2P file transfer (Tauri 2.0 + React 18)
  - WRAITH-Android: Native Kotlin + Jetpack Compose
  - WRAITH-iOS: Native Swift + SwiftUI
  - WRAITH-Chat: E2EE messaging with Double Ratchet
- ✅ Frontend test infrastructure (62 Vitest tests)
- ✅ Wayland compatibility fix (KDE Plasma 6 crash resolved)
- ✅ Tauri 2.0 capability-based permissions (plugin initialization fix)
- ⬜ 6 Tier 2/3 clients awaiting future development

**Completed Work:**

**Phase 15: Reference Client Foundation - ✅ COMPLETE (2025-12-09):**
1. ✅ FFI layer for wraith-core (C ABI bindings - wraith-ffi crate)
2. ✅ Tauri 2.0 desktop shell (IPC, window management)
3. ✅ React UI foundation (components, state, theme)
4. ✅ Transfer UI (file picker, progress, queue)

**Upcoming Work:**

**Phase 16: Mobile Clients (Planned Q2-Q4 2026):**
1. Android client (Kotlin + Jetpack Compose)
2. iOS client (Swift + SwiftUI)

**Tier 1 Remaining (Q2-Q3 2026):**
1. WRAITH-Chat beta (1-on-1 messaging, group chat)

**Next Steps:**
1. Review Phase 15 deliverables (WRAITH Transfer v1.5.8)
2. Begin Phase 16 sprint planning (mobile clients)
3. Set up shared component development (wraith-contacts, wraith-gui-common)
4. Configure cross-client CI/CD pipeline
5. Begin WRAITH-Chat design sprints

---

## Success Metrics (Planned)

### Technical Metrics

- [ ] All clients pass security audit (zero critical issues)
- [ ] Performance targets met (see individual client specifications)
- [ ] Cross-platform compatibility (Linux, macOS, Windows)
- [ ] Test coverage >80% (unit + integration)
- [ ] Documentation completeness: 100%

### Adoption Metrics (Post-Launch)

- [ ] 50K+ downloads (first 6 months)
- [ ] 1K+ active users (monthly)
- [ ] 500+ GitHub stars (WRAITH-Transfer)
- [ ] Community contributions (10+ PRs accepted)
- [ ] Production deployments (5+ case studies)

### Ecosystem Metrics

- [ ] All Tier 1 clients released (Transfer, Chat)
- [ ] 50%+ Tier 2 clients released (Sync or Share)
- [ ] 25%+ Tier 3 clients released (at least 2 of 4)
- [ ] Security testing clients released (with governance compliance)

---

## Links

- **Main README:** [../../README.md](../../README.md)
- **Protocol Development History:** [README_Protocol-DEV.md](README_Protocol-DEV.md)
- **Client Specifications:** [../clients/](../clients/)
- **Client Roadmap:** [../../to-dos/ROADMAP-clients.md](../../to-dos/ROADMAP-clients.md)
- **Protocol Roadmap:** [../../to-dos/ROADMAP.md](../../to-dos/ROADMAP.md)
- **CHANGELOG:** [../../CHANGELOG.md](../../CHANGELOG.md)
- **Security Testing Parameters:** [../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md](../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)

---

**WRAITH Protocol Client Applications Development History** - *From Planning to Production*

**Status:** Phases 15-16 Complete (All 4 Tier 1 Clients) | **Total Scope:** 10 clients, 1,148 SP | **Delivered:** 404 SP (35%) | **Remaining:** 744 SP (6 clients) | **Prerequisites:** Protocol v1.6.2 ✅ Complete | **Next:** Tier 2 Clients (WRAITH-Sync, WRAITH-Share)

*Last Updated: 2026-01-21*
