# Implementation Plan: WRAITH-RedOps Gap Remediation (v2.2.5)

This plan executes a methodical remediation of all findings in `GAP-ANALYSIS-v2.2.5.md`, prioritized by severity.

## Phase 1: High Priority (P1) - Core Stability Phase 1: High Priority (P1) - Core Stability & Crypto Crypto [checkpoint: ea65339]

- [ ] Task: Fix SMB2 Header Struct Compilation [NEW-17]
    - [ ] Write Tests: Add unit tests in `team-server` to verify `Smb2Header` field access.
    - [ ] Implement: Rename `reserved` to `process_id` and rename/fix `credit_request` usage in `team-server/src/listeners/smb.rs`.
- [ ] Task: Implement Double Ratchet Protocol [P1 #12]
    - [ ] Write Tests: Create property-based tests for KDF-chain rotation and DH ratchet.
    - [ ] Implement: Add `ratchet` module to `wraith-crypto`. Integrate Double Ratchet into `ProtocolHandler` (Team Server) and `Beacon` (Spectre).
- [ ] Task: Finalize PowerShell Runner Assembly [NEW-3]
    - [ ] Write Tests: Add a test in `spectre-implant` to verify the runner DLL can be dropped and loaded (mocking CLR).
    - [ ] Implement: Embed a functional, minimal C# assembly in `powershell.rs` and verify execution.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: High Priority' (Protocol in workflow.md)

## Phase 2: Medium Priority (P2) - Platform Completeness [checkpoint: 69cbd73]

- [ ] Task: Complete Operator IPC Bridge [NEW-18/19]
    - [ ] Write Tests: Add unit tests for each new IPC command in `operator-client/src-tauri`.
    - [ ] Implement: Register and wire 9 missing RPCs (Playbooks, RefreshToken, GetCampaign, etc.) in `lib.rs`.
- [ ] Task: Implement Dynamic Heap Discovery [P2 #19]
    - [ ] Write Tests: Add tests in `test_heap.rs` to verify discovery logic on both platforms.
    - [ ] Implement: Use `/proc/self/maps` parsing on Linux and `GetProcessHeap` on Windows in `obfuscation.rs`.
- [ ] Task: Native COM Persistence Implementation [NEW-8]
    - [ ] Write Tests: Add a `#[cfg(windows)]` test for `ITaskService` vtable resolution.
    - [ ] Implement: Define COM vtables and rewrite `install_scheduled_task` in `persistence.rs`.
- [ ] Task: Finalize Phishing VBA Runner [NEW-10]
    - [ ] Write Tests: Verify VBA template generation with valid shellcode runner macros.
    - [ ] Implement: Add `VirtualAlloc`/`CreateThread` logic to `phishing.rs`.
- [ ] Task: Miscellaneous P2 Remediation
    - [ ] Implement: Fix CLR GUIDs, integrate LLVM obfuscation RUSTFLAGS, and clean up `.unwrap()` calls in `c2/mod.rs`.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Medium Priority' (Protocol in workflow.md)

## Phase 3: Low Priority (P3) - Advanced Features Phase 3: Low Priority (P3) - Advanced Features & Quality Quality [checkpoint: aa5c56b]

- [ ] Task: Implement Structured P2P Mesh C2 [P3 #24]
    - [ ] Write Tests: Create a simulated mesh network test with 3+ beacons.
    - [ ] Implement: Build DHT-based or Tree-based routing logic in `wraith-discovery` and `spectre-implant`.
- [ ] Task: Tradecraft & UI Polish [NEW-12/13, P3 #28]
    - [ ] Implement: Persistent keylogger monitoring thread and dynamic ImageBase querying.
    - [ ] Implement: Add Settings UI for server address configuration in `App.tsx`.
- [ ] Task: Test Suite Expansion [NEW-20]
    - [ ] Implement: Expand unit and integration tests across all crates to reach >20% coverage.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Low Priority' (Protocol in workflow.md)
