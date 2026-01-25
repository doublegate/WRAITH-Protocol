# Implementation Plan - Remediate WRAITH-RedOps (Gap Analysis v3.0.0)

## Phase 1: Team Server Transport & Protocol (Gap 1.1, 1.4, 1.5, 1.7) [x]
- [x] Task: Implement WRAITH Protocol over UDP (Gap 1.1)
    - [x] Sub-task: Create `listeners/udp.rs` with `tokio::net::UdpSocket`.
    - [x] Sub-task: Port `Noise_XX` handshake logic from `http.rs` to a shared `session_manager` or util.
    - [x] Sub-task: Implement `handle_packet` in `udp.rs` using the shared handshake logic.
- [x] Task: Connect Task Delivery (Gap 1.4)
    - [x] Sub-task: Update `listeners/http.rs` `handle_beacon` to query `db.get_pending_commands`.
    - [x] Sub-task: Serialize commands into the `BeaconResponse` payload.
- [x] Task: Connect Real-time Events (Gap 1.5)
    - [x] Sub-task: Verify `main.rs` broadcast channel wiring.
    - [x] Sub-task: Update `services/operator.rs` `stream_events` to use the receiver correctly.
    - [x] Sub-task: Add event emission to `ImplantService` methods.
- [x] Task: Implement DNS & SMB Listeners (Gap 1.7)
    - [x] Sub-task: Implement `listeners/dns.rs` using `trust-dns-server` (or mock for now if crate missing, but actual logic).
    - [x] Sub-task: Implement `listeners/smb.rs` using named pipes (stub for Linux, conditional compile).
- [x] Task: Conductor - User Manual Verification 'Team Server Transport & Protocol' (Protocol in workflow.md)

## Phase 2: Governance & Security (Gap 1.2, 1.3) [x]
- [x] Task: Implement Kill Switch (Gap 1.2)
    - [x] Sub-task: Implement `services/killswitch.rs` with `UdpSocket` broadcast.
    - [x] Sub-task: Add API endpoint (gRPC) to trigger kill switch.
- [x] Task: Enhance Governance (Gap 1.3)
    - [x] Sub-task: Add `is_domain_allowed` to `governance.rs`.
    - [x] Sub-task: Implement HMAC signing for audit logs in `database/mod.rs`.
- [x] Task: Conductor - User Manual Verification 'Governance & Security' (Protocol in workflow.md)

## Phase 3: Spectre Implant Core & Evasion (Gap 3.1, 3.2, 3.3, 3.4) [x]
- [x] Task: Client-Side Noise Handshake (Gap 3.1)
    - [x] Sub-task: Implement `Noise_XX` initiator in `c2/mod.rs` using `snow` (no_std).
    - [x] Sub-task: Ensure `Elligator2` usage (if supported by snow/wrapper, else stub with comment).
- [x] Task: Advanced Evasion (Gap 3.2, 3.3)
    - [x] Sub-task: Implement ROP chain for `VirtualProtect` in `obfuscation.rs` (use assembly if needed).
    - [x] Sub-task: Implement `utils/syscalls.rs` with Windows SSN resolution (Hell's Gate).
    - [x] Sub-task: Connect `api_resolver.rs` `get_proc_address`.
- [x] Task: Conductor - User Manual Verification 'Spectre Implant Core & Evasion' (Protocol in workflow.md)

## Phase 4: Spectre Post-Exploitation (Gap 3.5, 3.6) [x]
- [x] Task: Implement Modules (Gap 3.5)
    - [x] Sub-task: Implement `modules/bof_loader.rs` (COFF parsing logic).
    - [x] Sub-task: Implement `modules/injection.rs` (logic for reflective injection).
    - [x] Sub-task: Implement `modules/socks.rs` (state machine for SOCKS5).
- [x] Task: Task Execution (Gap 3.6)
    - [x] Sub-task: Implement `dispatch_task` in `c2/mod.rs` to route commands to modules.
- [x] Task: Conductor - User Manual Verification 'Spectre Post-Exploitation' (Protocol in workflow.md)

## Phase 5: Operator Client & Builder (Gap 1.6, 2.1 - 2.4) [x]
- [x] Task: Builder Pipeline (Gap 1.6)
    - [x] Sub-task: Implement `builder/mod.rs` to patch binary configs.
    - [x] Sub-task: Expose builder via gRPC.
- [x] Task: Operator Client UI (Gap 2.1 - 2.4)
    - [x] Sub-task: Integrate `xterm.js` component.
    - [x] Sub-task: Add D3.js graph component.
    - [x] Sub-task: Create Campaign Wizard.
    - [x] Sub-task: Wire up missing IPC commands.
- [x] Task: Conductor - User Manual Verification 'Operator Client & Builder' (Protocol in workflow.md)

## Phase 6: Technical Debt & Final Polish [x]
- [x] Task: Cleanup Technical Debt
    - [x] Sub-task: Replace hardcoded secrets/IPs with env vars or config.
    - [x] Sub-task: Replace `.unwrap()` with `?` or `map_err`.
    - [x] Sub-task: Remove placeholder comments.
- [x] Task: Conductor - User Manual Verification 'Technical Debt Cleanup' (Protocol in workflow.md)
