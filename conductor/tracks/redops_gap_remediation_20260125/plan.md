# Implementation Plan: WRAITH-RedOps Remediation (Gap Analysis v2.2.5)

This plan outlines the methodical remediation of WRAITH-RedOps according to the Gap Analysis findings, following a strict Section-by-Section approach.

## Phase 1: Team Server Remediation (Gap Analysis Section 1) [checkpoint: 5f20813]

- [x] Task: 1.1 - Implement Operator ID extraction from gRPC metadata in `operator.rs` b32156e
    - [x] Write tests for gRPC metadata extraction
    - [x] Implement metadata interceptor/extraction logic
- [x] Task: 1.2 - Implement full DNS Tunneling Listener 07003c2
    - [x] Write tests for DNS protocol handling (TXT/A/AAAA)
    - [x] Implement DNS server logic and protocol handler in `dns.rs`
- [x] Task: 1.3 - Implement SMB Named Pipe Listener ddec603
    - [x] Write tests for SMB named pipe communication
    - [x] Implement SMB server logic in `smb.rs`
- [x] Task: 1.4 - Implement Implant Service Decryption and Binary Retrieval ab235f1
    - [x] Write tests for command/payload decryption
    - [x] Implement decryption logic and builder integration in `implant.rs`
- [x] Task: 1.5 - Connect HTTP Listener to Database and Tasks a0e7a89
    - [x] Write tests for task queuing and delivery via HTTP
    - [x] Implement DB query logic and Frame construction in `http.rs`
- [x] Task: 1.9 - Implement Full Implant Build Pipeline afd1bc4
    - [x] Write tests for dynamic implant compilation
    - [x] Implement LLVM build pipeline and obfuscation in `builder/mod.rs`
- [x] Task: Conductor - User Manual Verification 'Phase 1: Team Server' (Protocol in workflow.md) b32156e

## Phase 2: Spectre Implant Remediation (Gap Analysis Section 2) [checkpoint: 09b0b3c]

- [x] Task: 2.1 - Implement Process Injection Modules 6918dd0
    - [x] Write tests for reflective injection, hollowing, and thread hijacking
    - [x] Implement logic in `modules/injection.rs`
- [x] Task: 2.2 - Implement BOF Loader 2ed4b90
    - [x] Write tests for COFF loading and symbol resolution
    - [x] Implement full loader in `modules/bof_loader.rs`
- [x] Task: 2.3 - Implement SOCKS Proxy 102d755
    - [x] Write tests for SOCKS4a/5 authentication and proxying
    - [x] Implement logic in `modules/socks.rs`
- [x] Task: 2.4 - Implement PTY Shell and Fix C2 Hardcodings 5af2461
    - [x] Write tests for interactive shell execution
    - [x] Implement PTY shell in `c2/mod.rs` and remove hardcoded IPs
- [x] Task: 2.5 - Implement Halo's Gate SSN Resolution 253d070
    - [x] Write tests for syscall resolution
    - [x] Implement Halo's Gate logic in `utils/syscalls.rs`
- [x] Task: 2.6 - 2.9 - Complete Shell Module and Heap Discovery fc84660
    - [x] Write tests for shell command execution and runtime heap discovery
    - [x] Implement modules in `modules/shell.rs` and `utils/obfuscation.rs`
- [x] Task: Conductor - User Manual Verification 'Phase 2: Spectre Implant' (Protocol in workflow.md) 09b0b3c

## Phase 3: Operator Client & Integration (Gap Analysis Section 3)

- [x] Task: 3.1 - Implement Dashboard Metrics Backend Integration b3c5e9d
    - [x] Write tests for metrics aggregation and retrieval
    - [x] Implement data flows in `App.tsx` and Tauri handlers
- [x] Task: 3.4 - Implement Actual IPC Data Retrieval b3c5e9d
    - [x] Write tests for all IPC commands (list beacons, listeners, etc.)
    - [x] Replace `vec![]` stubs with real data in `operator-client/src-tauri/src/lib.rs`
- [ ] Task: Integration - End-to-End Command Encryption
    - [ ] Write integration tests for E2E encrypted tasking
    - [ ] Implement encryption at rest in DB and delivery to implant
- [ ] Task: Integration - Kill Switch Logic Implementation
    - [ ] Write tests for kill switch triggers
    - [ ] Implement verification logic in `killswitch.rs`
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Operator Client & Integration' (Protocol in workflow.md)
