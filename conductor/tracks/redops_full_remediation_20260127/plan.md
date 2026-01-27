# Plan: WRAITH-RedOps Full Remediation & Aspirational Integration

## Phase 1: Core Functionality & Protocol Acceleration
- [x] Task: Implement Noise DH Key Ratcheting [b4ff4f2]
    - [x] Write unit tests for rekeying state transitions and handshake counters.
    - [x] Implement DH ratchet logic in `team-server/src/services/protocol.rs`.
    - [x] Implement client-side rekeying in `spectre-implant/src/c2/mod.rs`.
- [x] Task: Integrate AF_XDP Kernel Bypass [26c6d86]
    - [x] Write unit tests for zero-copy UMEM and ring buffer management.
    - [x] Implement AF_XDP transport driver in `wraith-transport` for RedOps listeners.
    - [x] Update `team-server/src/listeners` to support AF_XDP accelerated sockets.
- [x] Task: Implement io_uring Asynchronous I/O [b984865]
    - [x] Write unit tests for completion queue submission and polling.
    - [x] Integrate `io-uring` crate for file/network I/O in the Team Server.
- [x] Task: Integrate BBR Congestion Control [421e91b9]
    - [x] Write tests for BBR bandwidth and RTT estimation logic.
    - [x] Implement BBR algorithm in the protocol session layer.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Core Functionality & Protocol Acceleration' (Protocol in workflow.md)

## Phase 2: PowerShell Execution & High-Priority Fixes
- [ ] Task: Deliver Production-Grade PowerShell Runner
    - [ ] Develop and compile a native C# .NET assembly for unmanaged PowerShell execution.
    - [ ] Embed the compiled runner into `spectre-implant/src/modules/powershell.rs`.
    - [ ] Write integration tests for in-memory PowerShell execution.
- [ ] Task: Resolve P2 High-Priority Bugfixes
    - [ ] Correct CLR GUID for `CLSID_CLRRuntimeHost` in `clr.rs`.
    - [ ] Replace `.unwrap()` calls in `team-server/src/listeners/smb.rs` with robust error handling.
    - [ ] Remove `HMAC_SECRET` fallback in `start_redops.sh` and add startup validation.
- [ ] Task: Complete VBA Phishing Payload
    - [ ] Implement the shellcode runner logic (VirtualAlloc/CreateThread) in `builder/phishing.rs`.
    - [ ] Write tests for VBA macro generation.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: PowerShell Execution & High-Priority Fixes' (Protocol in workflow.md)

## Phase 3: Platform Completeness & Evasion
- [ ] Task: Implement Dynamic Memory Discovery
    - [ ] Implement PEB parsing to find the actual `.text` section range in `obfuscation.rs`.
    - [ ] Implement runtime heap discovery (Windows: `GetProcessHeap`; Linux: `/proc/self/maps`).
- [ ] Task: Build-Time LLVM Obfuscation
    - [ ] Configure `builder/mod.rs` to apply LLVM-level obfuscation passes via `RUSTFLAGS`.
    - [ ] Write tests to verify symbol stripping and control flow flattening in generated binaries.
- [ ] Task: Hardware-Based ARM64 Entropy
    - [ ] Implement `CNTVCT_EL0` register reading for ARM64 entropy in `entropy.rs`.
- [ ] Task: Multi-Transport Failover
    - [ ] Write tests for transport state monitoring and failover triggers.
    - [ ] Implement autonomous transport switching logic in the Spectre C2 client.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Platform Completeness & Evasion' (Protocol in workflow.md)

## Phase 4: Advanced Features & Distributed Discovery
- [ ] Task: P2P Mesh C2 Orchestration
    - [ ] Implement mesh routing tables and automated peer discovery in `mesh.rs`.
    - [ ] Write tests for multi-hop command relaying through mesh nodes.
- [ ] Task: Kademlia DHT Integration
    - [ ] Integrate `wraith-discovery` DHT for decentralized RedOps peer discovery.
- [ ] Task: Advanced Persistence & Evasion
    - [ ] Implement persistent keylogger with configurable polling threads.
    - [ ] Implement PEB-based ImageBase queries for all injection modules.
- [ ] Task: Operator Experience (Tauri UI)
    - [ ] Develop a Settings/Preferences dashboard in the Operator Client.
    - [ ] Implement persistent storage for server addresses and operator keys.
- [ ] Task: Test Coverage Remediation (Backfill)
    - [ ] Expand unit test suites for all legacy modules to meet >80% coverage.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Advanced Features & Distributed Discovery' (Protocol in workflow.md)
