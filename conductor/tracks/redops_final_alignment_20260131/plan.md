# Implementation Plan: WRAITH-RedOps Final Alignment and Completion

This plan outlines the methodical remediation of all gaps identified in `GAP-ANALYSIS-v2.3.4.md` and the final implementation of remaining MITRE ATT&CK techniques.

## Phase 1: Spectre Implant (Backend & Tradecraft)

- [x] Task: Implement Signal-style Double Ratchet protocol (P1-1) fd17685
    - [x] Create failing integration test for DH + Symmetric ratcheting
    - [x] Implement `DoubleRatchet` state machine in `wraith-crypto`
    - [x] Update `NoiseTransport` to support ratchet steps
    - [x] Integrate into `spectre-implant` C2 loop
    - [x] Verify 100% branch coverage for crypto paths
- [x] Task: Transition PowerShell Runner to Source-Build (P1-2) d3d9a1e
    - [x] Integrate C# source code into `runner_src/`
    - [x] Add `dotnet build` step to project `xtask` or helper script
    - [x] Update `powershell.rs` to include the freshly built `Runner.dll`
    - [x] Write tests verifying managed code execution
- [ ] Task: Implement Advanced MITRE ATT&CK Techniques
    - [ ] T1059.003: Managed Windows Command Shell execution
    - [ ] T1134: Access Token Manipulation (Impersonation, Creation, Delegation)
    - [ ] T1140: In-memory Deobfuscate/Decode module
    - [ ] T1574.002: DLL Side-Loading identification and exploit module
    - [ ] T1105: Multi-protocol Ingress Tool Transfer (HTTP/S, SMB, DNS)
- [ ] Task: Remediate P2/P3 Implant Findings
    - [ ] Verify and update CLR MetaHost CLSID (P2-2)
    - [ ] Implement Browser DPAPI decryption (P3-1)
    - [ ] Implement dynamic Linux .text base address calculation (P3-2)
    - [ ] Obfuscate UDP Mesh discovery signatures (P3-3)
    - [ ] Implement missing Windows SMB client functionality (P3-4)
    - [ ] Upgrade compression module to zlib/deflate (P3-5)
    - [ ] Implement safe error handling for BOF parser (P3-6)
- [ ] Task: Conductor - User Manual Verification 'Spectre Implant' (Protocol in workflow.md)

## Phase 2: Team Server (Infrastructure & Protocol)

- [ ] Task: Integrate Double Ratchet into Protocol Handler
    - [ ] Update `team-server/src/services/protocol.rs` to handle ratchet handshake frames
    - [ ] Implement per-message symmetric ratchet step in server session state
    - [ ] Write integration tests for multi-message ratcheted sessions
- [ ] Task: Enhance Infrastructure Safety and Robustness
    - [ ] Implement graceful error handling for Kill Switch env vars (P2-3)
    - [ ] Implement cryptographically secure nonce generation for response frames (P2-5)
    - [ ] Verify zero warnings across all server modules
- [ ] Task: Conductor - User Manual Verification 'Team Server' (Protocol in workflow.md)

## Phase 3: Operator Client (Frontend & UX)

- [ ] Task: Complete Console Command Mapping (P2-1)
    - [ ] Add UI handlers for `compress`, `exfil_dns`, `wipe`, and `hijack` in `Console.tsx`
    - [ ] Verify input validation for each new command
- [ ] Task: Implement Resource Management (Delete)
    - [ ] Add "Delete Listener" functionality to `ListenerManager.tsx`
    - [ ] Add "Delete Attack Chain" functionality to `PlaybookBrowser.tsx`
- [ ] Task: UI/UX Polish and Advanced Features
    - [ ] Global version string update to `v2.3.4`
    - [ ] Implement multi-implant bulk operations in Beacons view
    - [ ] Add Dark/Light theme toggle support
- [ ] Task: Conductor - User Manual Verification 'Operator Client' (Protocol in workflow.md)

## Phase 4: Final Integration & Release Prep

- [ ] Task: Full Platform Verification
    - [ ] Execute complete `cargo xtask ci` suite
    - [ ] Verify >80% coverage for all modified modules
    - [ ] Perform end-to-end mission rehearsal (Implant -> Server -> Client)
- [ ] Task: Final Documentation and Audit
    - [ ] Update `CHANGELOG.md` with v2.3.4 release notes
    - [ ] Final security audit of all new cryptographic paths
- [ ] Task: Conductor - User Manual Verification 'Final Release' (Protocol in workflow.md)
