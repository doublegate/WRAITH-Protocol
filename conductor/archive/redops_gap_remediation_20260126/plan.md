# Implementation Plan: WRAITH-RedOps Gap Remediation (v2.2.5)

This plan methodically addresses all gaps identified in the v2.2.5 Gap Analysis, structured by priority to ensure critical security and core features are remediated first.

## Phase 1: P1 High Priority Gaps (Core Phase 1: P1 High Priority Gaps (Core & Security) Security) [checkpoint: 53c6444]

- [x] **Task: Implement Noise DH Key Ratcheting** (53c6444)
    - [ ] **Write Tests:** Create a test in `protocol.rs` that simulates a session reaching 1M packets or 2 minutes and asserts that a re-keying event is triggered.
    - [ ] **Implement:** Add DH ratcheting logic to the Team Server protocol handler to rotate keys based on specified thresholds.
- [x] **Task: Implement Real PowerShell Runner Assembly** (53c6444)
    - [ ] **Implement:** Replace the `RUNNER_DLL` MZ placeholder in `powershell.rs` with a functional, minimal .NET assembly capable of executing PowerShell scripts.
    - [ ] **Linux Validation:** Create a standalone harness `test_ps_runner` to verify that Spectre correctly handles the dropping and attempted execution of the runner.
- [x] **Task: Wire Attack Chain IPC Bridge (Operator Client)** (53c6444)
    - [ ] **Implement:** Register `create_attack_chain`, `list_attack_chains`, `execute_attack_chain`, and `get_attack_chain` commands in `src-tauri/src/lib.rs`.
    - [ ] **Implement:** Map these IPC commands to the corresponding gRPC client calls in the Tauri backend.
- [ ] **Task: Conductor - User Manual Verification 'Phase 1: P1 High Priority Gaps (Core Phase 1: P1 High Priority Gaps (Core & Security) Security) [checkpoint: 53c6444]' (Protocol in workflow.md)**

## Phase 2: P2 Medium Priority Gaps (Platform Completeness) [checkpoint: fbe9007]

- [~] **Task: DNS Multi-Label Encoding - [ ] **Task: DNS Multi-Label Encoding & TXT Formatting** TXT Formatting**
    - [ ] **Write Tests:** Add unit tests in `dns.rs` for multi-label subdomain parsing and TXT record RDATA validity.
    - [ ] **Implement:** Support chunked payload encoding across multiple labels in the DNS listener.
    - [ ] **Implement:** Fix TXT record wrapping in Spectre's DNS module to ensure valid RDATA.
- [x] **Task: Dynamic Heap Discovery - [ ] **Task: Dynamic Heap Discovery & CLR GUID Fix** CLR GUID Fix**
    - [ ] **Implement:** Replace hardcoded heap addresses in `obfuscation.rs` with runtime discovery via `/proc/self/maps` (Linux) and dynamic API calls (Windows).
    - [ ] **Implement:** Correct the CLSID for `CLRRuntimeHost` in `clr.rs`.
    - [ ] **Linux Validation:** Use a standalone script `test_heap_discovery` to confirm correct memory range detection on the host.
- [~] **Task: Handshake Robustness - [ ] **Task: Handshake Robustness & .unwrap() Cleanup** .unwrap() Cleanup**
    - [ ] **Implement:** Replace `.unwrap()` and `.expect()` calls in `c2/mod.rs` handshake sequence with proper error propagation.
    - [ ] **Implement:** Standardize error handling across all Spectre modules to meet production-ready standards.
- [x] **Task: LLVM Obfuscation - [ ] **Task: LLVM Obfuscation & VBA Phishing Runner** VBA Phishing Runner** (fbe9007)
    - [ ] **Implement:** Update the builder pipeline to inject actual `RUSTFLAGS` for LLVM obfuscation passes.
    - [ ] **Implement:** Complete the VBA shellcode runner (`CreateThread`/`VirtualAlloc`) in the phishing macro generator.
- [x] **Task: Native COM Scheduled Task Persistence** (fbe9007)
    - [ ] **Implement:** Implement the full `ITaskService` COM vtable in `persistence.rs` to remove the fallback to `schtasks.exe`.
- [x] **Task: AttackChainEditor Real-World Wiring** (fbe9007)
    - [ ] **Implement:** Remove `setInterval` simulation from `AttackChainEditor.tsx`.
    - [ ] **Implement:** Use `invoke()` to connect "Save" and "Execute" actions to the newly wired IPC bridge.
- [ ] **Task: Conductor - User Manual Verification 'Phase 2: P2 Medium Priority Gaps (Platform Completeness) [checkpoint: fbe9007]' (Protocol in workflow.md)**

## Phase 3: P3 Low Priority Gaps Phase 3: P3 Low Priority Gaps & Polish Polish [checkpoint: 7748ea2]

- [x] **Task: Initial P2P Mesh C2 Implementation** (fbe9007)
    - [ ] **Write Tests:** Test basic mesh routing between two simulated implants.
    - [ ] **Implement:** Add mesh relay logic to the Team Server and Spectre's beacon loop.
- [x] **Task: APT Playbook Sequence Implementation** (Skipped)
    - [ ] **Implement:** Add support for predefined technique sequences (Playbooks) in the attack chain backend.
- [x] **Task: SMB2 Protocol Hardening** (Skipped)
    - [ ] **Implement:** Transition SMB listener and client from length-prefix framing to a structured SMB2-style header implementation.
- [ ] **Task: Settings UI & Keylogger Persistence**
    - [ ] **Implement:** Add a Settings page to the Operator Console for configuring server addresses and UI preferences.
    - [ ] **Implement:** Move the Spectre keylogger to a persistent monitoring model with a configurable interval.
- [x] **Task: Process Hollowing PEB Query** (7748ea2)
    - [ ] **Implement:** Update process hollowing logic to query the PEB for the target's actual `ImageBase` instead of assuming `0x400000`.
- [ ] **Task: Conductor - User Manual Verification 'Phase 3: P3 Low Priority Gaps Phase 3: P3 Low Priority Gaps & Polish Polish [checkpoint: 7748ea2]' (Protocol in workflow.md)**
