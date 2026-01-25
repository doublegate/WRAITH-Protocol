# Implementation Plan - Comprehensive Finalization of WRAITH-RedOps

## Phase 1: Spectre Implant Finalization - Part A (Gap 3.4, 3.5, 3.8) [x]
- [x] Task: Implement Native PTY/Pipe Shell Module
    - [x] Sub-task: Write unit tests for PTY master/slave allocation (Linux) and Anonymous Pipe redirection (Windows).
    - [x] Sub-task: Implement `modules/shell.rs` with full I/O redirection and process management.
- [x] Task: Implement Production-Grade COFF/BOF Loader
    - [x] Sub-task: Write exhaustive tests for COFF relocation types (DIR64, REL32) using sample object files.
    - [x] Sub-task: Implement manual section mapping and memory permission management (`VirtualProtect`).
    - [x] Sub-task: Implement full Beacon API resolution table (Printf, Output, Token manipulation).
    - [x] Sub-task: Implement the COFF loader in `modules/bof_loader.rs` ensuring `no_std` compliance.
- [x] Task: Conductor - User Manual Verification 'Spectre Implant Finalization - Part A' (Protocol in workflow.md)

## Phase 2: Spectre Implant Finalization - Part B (Gap 3.3, 3.6, 3.7) [x]
- [x] Task: Implement strictly Indirect Syscall Injection Suite
    - [x] Sub-task: Write unit tests for dynamic SSN resolution from `ntdll.dll` memory.
    - [x] Sub-task: Implement `NtCreateSection`, `NtMapViewOfSection`, `NtCreateThreadEx` wrappers in `utils/syscalls.rs`.
    - [x] Sub-task: Implement Reflective DLL Injection using purely indirect syscalls.
    - [x] Sub-task: Implement Process Hollowing using purely indirect syscalls.
- [x] Task: Implement Full SOCKS5 Proxy State Machine
    - [x] Sub-task: Write unit tests for SOCKS5 greeting, authentication, and connection negotiation.
    - [x] Sub-task: Implement data multiplexing logic to tunnel traffic through the Noise-encrypted C2 channel.
    - [x] Sub-task: Implement `modules/socks.rs` fulfilling the RFC 1928 specification.
- [x] Task: Conductor - User Manual Verification 'Spectre Implant Finalization - Part B' (Protocol in workflow.md)

## Phase 3: Team Server & Operator Finalization (Gap 1.3, 1.6, 1.7, 2.1, 2.2) [x]
- [x] Task: Finalize DNS Tunneling & SMB Routing
    - [x] Sub-task: Write unit tests for frame fragmentation/reassembly and multi-hop routing logic.
    - [x] Sub-task: Implement TXT record assembly in `listeners/dns.rs`.
    - [x] Sub-task: Implement session-linked SMB pivoting in `services/protocol.rs`.
- [x] Task: Implement Runtime Compilation Builder
    - [x] Sub-task: Write tests for dynamic `cargo` invocation and LLVM flag injection.
    - [x] Sub-task: Implement `builder/mod.rs` to compile unique implants from source with obfuscation passes.
- [x] Task: Finalize Operator Dashboard & Settings
    - [x] Sub-task: Implement real-time telemetry stream consumption in the frontend.
    - [x] Sub-task: Implement the Settings Manager and Dashboard metrics components.
- [x] Task: Conductor - User Manual Verification 'Team Server & Operator Finalization' (Protocol in workflow.md)
