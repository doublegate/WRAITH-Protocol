# Implementation Plan: WRAITH-RedOps Final Remediation & Completion

This plan outlines the final development sprint to bring WRAITH-RedOps to 100% completion, addressing critical security gaps, core functionality omissions, and full MITRE ATT&CK tradecraft integration.

## Phase 1: P0 Critical Safety & Security [checkpoint: ab82c23]

- [x] Task: Remediate Hardcoded Cryptographic Fallbacks
    - [x] Update `database/mod.rs` to require `HMAC_SECRET` and `MASTER_KEY` via `.expect()`
    - [x] Update `services/killswitch.rs` to load key seed from environment or secure storage
- [x] Task: Implement gRPC Channel Security & Auth
    - [x] Implement a gRPC interceptor in the Team Server to enforce JWT authentication on all OperatorService routes
    - [x] Update `OperatorServiceImpl::authenticate` to perform real Ed25519 signature verification against the operator's public key
- [x] Task: Conductor - User Manual Verification 'Phase 1: P0 Security' (Protocol in workflow.md) ab82c23

## Phase 2: P1 High Priority Core Implementation

- [~] Task: Complete Windows Injection & Post-Exploitation
    - [ ] Implement thread enumeration and full Thread Hijack in `modules/injection.rs`
    - [ ] Implement proper Process Hollowing (NtUnmapViewOfSection) in `modules/injection.rs`
    - [ ] Implement BOF IAT resolution and full BIF (BeaconPrintf/DataParse) in `modules/bof_loader.rs`
- [ ] Task: Enhance C2 Communication & Transport
    - [ ] Implement Noise session key ratcheting in `protocol.rs` and `session.rs`
    - [ ] Implement actual TCP Relay for SOCKS proxy in `modules/socks.rs`
    - [ ] Implement task dispatch for ALL module types in `c2/mod.rs`
- [ ] Task: Implement Dynamic Listener Management
    - [ ] Update `start_listener`/`stop_listener` logic to spawn and kill async tokio tasks for each listener type
- [ ] Task: Conductor - User Manual Verification 'Phase 2: P1 Core' (Protocol in workflow.md)

## Phase 3: P2 Medium Priority Completeness

- [ ] Task: Implement Linux Implant Parity
    - [ ] Implement Reflective, Hollowing, and Thread Hijack for Linux in `modules/injection.rs`
    - [ ] Implement Halo's Gate SSN resolution in `utils/syscalls.rs`
    - [ ] Implement runtime heap discovery in `utils/obfuscation.rs`
- [ ] Task: Team Server Listener & Storage Hardening
    - [ ] Implement multi-label subdomain encoding for DNS Tunneling
    - [ ] Implement XChaCha20-Poly1305 encryption for Artifacts in `database/mod.rs`
    - [ ] Externalize listener ports in `main.rs` to environment variables
- [ ] Task: Production Error Handling Cleanup
    - [ ] Audit and replace `unwrap()` calls in production paths with structured error handling
- [ ] Task: Conductor - User Manual Verification 'Phase 3: P2 Completeness' (Protocol in workflow.md)

## Phase 4: P3 Low Priority & Advanced Implementation [checkpoint: 0244497]

- [x] Task: Advanced Evasion & Automation
    - [x] Implement Sleep Mask (ROP-based section encryption) in `utils/obfuscation.rs`
    - [x] Implement P2P Mesh routing logic in the Team Server and Implant
    - [x] Implement APT Playbook engine for technique sequencing
- [x] Task: Protocol & UI Finalization
    - [x] Implement full SMB2 protocol headers for the SMB listener
    - [x] Add Settings UI to the Operator Client for server address management
- [x] Task: Conductor - User Manual Verification 'Phase 4: P3 Finalization' (Protocol in workflow.md) 0244497

## Phase 5: MITRE ATT&CK Tradecraft Integration [checkpoint: 3a927e8]

- [x] Task: TA0001 & TA0002 (Access & Execution) 77c44cc
    - [x] Implement Phishing Payload Generator (T1566) in Team Server
    - [x] Implement Unmanaged PowerShell Host (T1059) in Spectre
- [x] Task: TA0003 & TA0004 (Persistence & PrivEsc) 7e24479
    - [x] Implement Persistence Module (Registry/Task) (T1547/T1053)
    - [x] Implement UAC Bypass (Fodhelper) (T1548)
- [x] Task: TA0005 Defense Evasion 8cf9d72
    - [x] Implement Timestomp (T1070)
    - [x] Implement Sandbox Evasion Checks (T1497)
- [x] Task: TA0006 & TA0007 (Credential Access & Discovery) 37e92bb
    - [x] Implement LSASS Minidump (T1003) via direct syscalls
    - [x] Implement Native API Discovery (NetScan/SysInfo) (T1082/T1087)
- [x] Task: TA0008 & TA0009 & TA0040 (Lateral Movement, Collection, Impact) d24297e
    - [x] Implement WMI/PsExec Lateral Movement (T1021)
    - [x] Implement Screenshot & Keylogger (T1113/T1056)
    - [x] Implement Service Stop (T1489)
- [x] Task: Conductor - User Manual Verification 'Phase 5: MITRE ATT&CK' (Protocol in workflow.md) 3a927e8
