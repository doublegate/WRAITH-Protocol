# Implementation Plan: WRAITH-RedOps Implementation Completion

This plan follows the 7-Sprint remediation timeline defined in the Gap Analysis v4.1.0 to finish the implementation of the RedOps platform.

## Phase 1: Sprint 1 - P0 Critical Security [checkpoint: 3fe2c30]

- [x] **Task: Fix gRPC Authentication Passthrough** (8f6441d)
    - [ ] Update `auth_interceptor` in `team-server/src/main.rs` to reject requests missing the `authorization` header.
    - [ ] Add a whitelist for the `Authenticate` RPC method to allow initial login.
    - [ ] Verify that unauthenticated requests to protected endpoints now return `Status::unauthenticated`.
- [ ] **Task: Conductor - User Manual Verification 'Phase 1: Sprint 1' (Protocol in workflow.md)**

## Phase 2: Sprint 2 - P1 Core Bugs & Gaps

- [ ] **Task: Fix CONTEXT Struct Structural Bug**
    - [ ] Move orphaned field declarations into the `CONTEXT` struct body in `spectre-implant/src/utils/windows_definitions.rs`.
    - [ ] Verify `size_of::<CONTEXT>()` matches Windows expectations (approx. 1232 bytes for x64).
- [ ] **Task: Externalize Kill Signal Configuration**
    - [ ] Update `operator.rs` to read the kill signal port and secret from environment variables or campaign configuration instead of using hardcoded values.
- [ ] **Task: Implement PowerShell Runner (Non-Stub)**
    - [ ] Replace `MZ_PLACEHOLDER` in `powershell.rs` with a valid .NET runner assembly or a mechanism to load it.
- [ ] **Task: Implement BeaconDataParse BIF**
    - [ ] Implement the argument parsing logic for Cobalt Strike BOF compatibility in `bof_loader.rs`.
- [ ] **Task: Conductor - User Manual Verification 'Phase 2: Sprint 2' (Protocol in workflow.md)**

## Phase 3: Sprint 3 - P1 C2 Expansion

- [ ] **Task: Implement Full Task Dispatch in Implant**
    - [ ] Add handlers for `inject`, `bof`, and `socks` task types in `spectre-implant/src/c2/mod.rs`.
- [ ] **Task: Implement SOCKS TCP Relay**
    - [ ] Replace the simulation logic in `socks.rs` with actual asynchronous TCP relaying to the target host.
- [ ] **Task: Implement Noise Key Ratcheting**
    - [ ] Add logic to update the Noise session keys every 2 minutes or 1,000,000 packets as per protocol specification.
- [ ] **Task: Conductor - User Manual Verification 'Phase 3: Sprint 3' (Protocol in workflow.md)**

## Phase 4: Sprint 4 - P1 Dynamic Management + Beacon Data

- [ ] **Task: Implement Dynamic Listener Spawning**
    - [ ] Update `start_listener` and `stop_listener` in `team-server/src/services/operator.rs` to spawn/abort active Tokio tasks for each listener.
- [ ] **Task: Populate Real Beacon Data**
    - [ ] Replace static JSON in the implant check-in with dynamic system metadata (hostname, user, arch, etc.).
- [ ] **Task: Conductor - User Manual Verification 'Phase 4: Sprint 4' (Protocol in workflow.md)**

## Phase 5: Sprint 5 - P2 Platform & Stubs

- [ ] **Task: Implement Linux Injection and Discovery**
    - [ ] Implement `process_vm_writev` or `ptrace` logic for injection and `uname`/`/proc` parsing for discovery on Linux.
- [ ] **Task: Implement Credential Dumping Logic**
    - [ ] Implement LSASS memory parsing or a similar mechanism for credential harvesting on Windows.
- [ ] **Task: Implement Network Connect Scanner**
    - [ ] Implement the TCP connect scan logic in the `discovery` module.
- [ ] **Task: Randomize Sleep Mask XOR Key**
    - [ ] Modify `obfuscation.rs` to use session-derived or random XOR keys for the sleep mask instead of the hardcoded `0xAA`.
- [ ] **Task: Conductor - User Manual Verification 'Phase 5: Sprint 5' (Protocol in workflow.md)**

## Phase 6: Sprint 6 - P2 Completeness

- [ ] **Task: Enhance DNS Multi-Label Encoding**
    - [ ] Implement chunked encoding across multiple subdomain labels for larger payloads in the DNS listener.
- [ ] **Task: Implement Artifact Encryption at Rest**
    - [ ] Apply XChaCha20-Poly1305 encryption to artifact content before storage in the database.
- [ ] **Task: Externalize Listener Ports**
    - [ ] Move hardcoded ports (8080, 9999, etc.) from `main.rs` to environment variables.
- [ ] **Task: Implement Native Persistence APIs**
    - [ ] Replace shell-based persistence methods with native API calls (e.g., Task Scheduler COM API).
- [ ] **Task: Conductor - User Manual Verification 'Phase 6: Sprint 6' (Protocol in workflow.md)**

## Phase 7: Sprint 7 - P3 Advanced Features

- [ ] **Task: Implement ROP Sleep Mask**
    - [ ] Implement `.text` section encryption/decryption using a ROP-based sleep mask.
- [ ] **Task: Implement P2P Mesh C2**
    - [ ] Add support for SMB/TCP peer-to-peer beacon routing in the implant and team server.
- [ ] **Task: Implement Keylogger Mapping and Persistence**
    - [ ] Complete the virtual key mapping and implement a persistent monitoring buffer for the keylogger.
- [ ] **Task: Full SMB2 Protocol Headers**
    - [ ] Upgrade the SMB listener and implant transport to use full SMB2 protocol headers.
- [ ] **Task: Conductor - User Manual Verification 'Phase 7: Sprint 7' (Protocol in workflow.md)**
