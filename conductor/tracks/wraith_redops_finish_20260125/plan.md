# Implementation Plan - Remediate WRAITH-RedOps

## Phase 1: Team Server Core & Protocol Integration (Gap 1.1, 1.4, 1.5)
- [x] Task: Integrate WRAITH Protocol (Gap 1.1)
    - [x] Sub-task: Add `wraith-crypto` and `wraith-transport` dependencies to `team-server`.
    - [x] Sub-task: Implement `Noise_XX` session management in `services/mod.rs` (or appropriate location).
    - [x] Sub-task: Update `http.rs` listener to handle encrypted WRAITH frames (mimicry).
    - [x] Sub-task: Implement UDP listener in `listeners/udp.rs` using `wraith-transport`.
- [x] Task: Listener Management (Gap 1.4)
    - [x] Sub-task: Create `listeners/dns.rs` (stub/basic impl) and `listeners/smb.rs` (stub/basic impl) to satisfy architecture.
    - [x] Sub-task: Implement full lifecycle management (start/stop) for all listener types in `OperatorService`.
- [x] Task: Real-time Events (Gap 1.5)
    - [x] Sub-task: Connect `broadcast::Sender` to `ImplantService` actions (check-in, task result).
    - [x] Sub-task: Ensure `stream_events` gRPC endpoint correctly yields these events.
- [~] Task: Conductor - User Manual Verification 'Team Server Core & Protocol Integration' (Protocol in workflow.md)

## Phase 2: Governance & Security (Gap 1.2)
- [ ] Task: Scope Enforcement (Gap 1.2)
    - [ ] Sub-task: Enhance `GovernanceEngine` to support CIDR whitelist/blacklists.
    - [ ] Sub-task: Add middleware/interceptor to check all Implant actions against Scope.
- [ ] Task: Kill Switch & TTL (Gap 1.2)
    - [ ] Sub-task: Implement UDP broadcast mechanism for Kill Switch.
    - [ ] Sub-task: Add Time-to-Live checks in `ImplantService` check-in logic.
- [ ] Task: Audit Logging (Gap 1.2)
    - [ ] Sub-task: Ensure all critical actions (tasking, config changes) are written to `activity_log` with signatures.
- [ ] Task: Conductor - User Manual Verification 'Governance & Security' (Protocol in workflow.md)

## Phase 3: Spectre Implant Core Features (Gap 3.1, 3.2, 3.3)
- [ ] Task: WRAITH C2 Integration (Gap 3.1)
    - [ ] Sub-task: Ensure `spectre-implant` `c2/mod.rs` fully implements `Noise_XX` handshake using `snow` (aligned with Team Server).
    - [ ] Sub-task: Verify `WraithFrame` serialization/deserialization.
- [ ] Task: Evasion Features (Gap 3.2, 3.3)
    - [ ] Sub-task: Enhance `obfuscation.rs` to support ROP-based Sleep Mask (if feasible in no_std Rust without assembly, otherwise stub with detailed comments on ASM requirement).
    - [ ] Sub-task: Verify `syscalls.rs` Windows implementation (Hell's Gate) is complete and correct.
- [ ] Task: Task Execution (Gap 3.7)
    - [ ] Sub-task: Implement command dispatcher in `c2/mod.rs` to handle standard task types (shell, upload, download).
- [ ] Task: Conductor - User Manual Verification 'Spectre Implant Core Features' (Protocol in workflow.md)

## Phase 4: Advanced Implant Features (Gap 3.4, 3.5, 3.6)
- [ ] Task: Post-Exploitation Modules
    - [ ] Sub-task: Create `modules/bof_loader.rs` and implement COFF parser (Gap 3.4).
    - [ ] Sub-task: Create `modules/injection.rs` and implement injection logic (Gap 3.5).
    - [ ] Sub-task: Create `modules/socks.rs` and implement SOCKS proxy state machine (Gap 3.6).
- [ ] Task: Conductor - User Manual Verification 'Advanced Implant Features' (Protocol in workflow.md)

## Phase 5: Operator Client Enhancements (Gap 2.1 - 2.4)
- [ ] Task: Interactive Console (Gap 2.1)
    - [ ] Sub-task: Implement `xterm.js` component in `operator-client/src/components/Console.tsx`.
    - [ ] Sub-task: Wire up console input to `SendCommand` IPC.
- [ ] Task: Graph Visualization (Gap 2.2)
    - [ ] Sub-task: Implement D3.js beacon graph in `operator-client/src/components/NetworkGraph.tsx`.
- [ ] Task: Campaign & IPC (Gap 2.3, 2.4)
    - [ ] Sub-task: Create Campaign Wizard UI.
    - [ ] Sub-task: Implement missing IPC commands (`download_artifact`, listener controls) in `lib.rs` and frontend.
- [ ] Task: Conductor - User Manual Verification 'Operator Client Enhancements' (Protocol in workflow.md)

## Phase 6: Builder Pipeline (Gap 1.3)
- [ ] Task: Builder Implementation
    - [ ] Sub-task: Create `team-server/src/builder/` module.
    - [ ] Sub-task: Implement config patching logic (search and replace magic bytes in template binary).
    - [ ] Sub-task: Expose builder functionality via gRPC/API.
- [ ] Task: Conductor - User Manual Verification 'Builder Pipeline' (Protocol in workflow.md)
