# Implementation Plan - Finalize WRAITH-RedOps Implementation

## Phase 1: Spectre Implant Finalization (Gap 3.4, 3.5, 3.6, 3.8) [x]
- [x] Task: Implement COFF BOF Loader
    - [x] Sub-task: Write unit tests for COFF parsing and relocation handling in `no_std` context.
    - [x] Sub-task: Implement core COFF loader in `modules/bof_loader.rs`.
    - [x] Sub-task: Implement symbol resolution for standard Beacon APIs (BeaconPrintf, etc.).
- [x] Task: Implement Indirect Syscall Injection Suite
    - [x] Sub-task: Write unit tests for dynamic SSN resolution and indirect syscall execution.
    - [x] Sub-task: Refactor `modules/injection.rs` to use Hell's Gate/Halo's Gate for all process/memory operations.
    - [x] Sub-task: Implement Reflective DLL, Process Hollowing, and Thread Hijacking modules.
- [x] Task: Finalize SOCKS5 Proxy
    - [x] Sub-task: Write unit tests for SOCKS5 state machine (Auth, Connect, Data).
    - [x] Sub-task: Implement full SOCKS5 logic in `modules/socks.rs`.
- [x] Task: Implement Interactive Shell
    - [x] Sub-task: Write unit tests for PTY/Pipe communication handling.
    - [x] Sub-task: Implement interactive shell module for Windows and Linux.
- [x] Task: Conductor - User Manual Verification 'Spectre Implant Finalization' (Protocol in workflow.md)

## Phase 2: Team Server Finalization (Gap 1.3, 1.6, 1.7) [x]
- [x] Task: Implement SMB Pivoting Logic
    - [x] Sub-task: Write unit tests for parent-child session routing and packet multiplexing.
    - [x] Sub-task: Update `services/protocol.rs` to support routing WRAITH frames over session-linked SMB pipes.
    - [x] Sub-task: Update `listeners/smb.rs` to handle pivot registration and communication.
- [x] Task: Complete DNS Tunneling
    - [x] Sub-task: Write unit tests for frame fragmentation and reassembly via DNS TXT records.
    - [x] Sub-task: Implement full frame assembly logic in `listeners/dns.rs`.
- [x] Task: Implement Advanced Builder Pipeline
    - [x] Sub-task: Write unit tests for the LLVM build wrapper and config patching.
    - [x] Sub-task: Implement LLVM-based build logic in `builder/mod.rs` with basic obfuscation passes.
- [x] Task: Conductor - User Manual Verification 'Team Server Finalization' (Protocol in workflow.md)

## Phase 3: Operator Client Finalization (Gap 2.1, 2.2) [x]
- [x] Task: Implement Operational Dashboard
    - [x] Sub-task: Write tests for real-time metric collection and event stream consumption.
    - [x] Sub-task: Implement Dashboard UI component with active beacon and task metrics.
- [x] Task: Settings & Configuration UI
    - [x] Sub-task: Implement a settings manager to externalize Team Server connection parameters.
- [x] Task: Conductor - User Manual Verification 'Operator Client Finalization' (Protocol in workflow.md)
