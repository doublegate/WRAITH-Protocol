# Implementation Plan: WRAITH-RedOps Zero-Stub Completion

This plan enforces a strict "Zero Stub" policy for WRAITH-RedOps, replacing all placeholders with production-ready logic and updating the workflow guidelines to maintain this standard.

## Phase 1: Workflow Phase 1: Workflow & Guidelines Guidelines [checkpoint: ba7bf34]

- [~] **Task: Update Workflow Guidelines**
    - [ ] Modify `conductor/workflow.md` to add "Zero Stub" policy as Rule #1.
    - [ ] Explicitly forbid placeholders, skeleton implementations, or "coming soon" stubs in RedOps code.
- [ ] **Task: Conductor - User Manual Verification 'Phase 1: Workflow Phase 1: Workflow & Guidelines Guidelines [checkpoint: ba7bf34]' (Protocol in workflow.md)**

## Phase 2: Spectre Discovery & Collection

- [ ] **Task: Implement Full Linux System Discovery**
    - [ ] Replace `sys_info` stub in `modules/discovery.rs` with real `uname` and `/proc` parsing.
    - [ ] **Write Tests:** Create unit tests to verify real system data is returned on Linux.
    - [ ] **Implement:** Use raw syscalls to fetch hostname, kernel version, and architecture.
- [ ] **Task: Implement Full Network Scanner (Linux/Windows)**
    - [ ] Implement `net_scan` logic using raw socket syscalls (Linux) and Winsock (Windows).
    - [ ] **Write Tests (Linux):** Verify scanner can detect open ports on localhost.
    - [ ] **Implement:** Build a synchronous TCP connect scanner.
- [ ] **Task: Complete Keylogger Implementation**
    - [ ] Ensure `keylogger` in `modules/collection.rs` is persistent and handles buffer management.
    - [ ] **Implement:** Use `SetWindowsHookEx` (Windows) or `/dev/input` (Linux - optional based on perms).
- [ ] **Task: Conductor - User Manual Verification 'Phase 2: Spectre Discovery & Collection' (Protocol in workflow.md)**

## Phase 3: Spectre Injection & Lateral Movement

- [ ] **Task: Implement Robust Process Hollowing**
    - [ ] Revisit `modules/injection.rs` to ensure full implementation of process hollowing.
    - [ ] **Implement (Windows):** `CreateProcess`, `NtUnmapViewOfSection`, `VirtualAllocEx`, `WriteProcessMemory`, `SetThreadContext`, `ResumeThread`.
    - [ ] **Implement (Linux):** `ptrace` based injection or similar.
- [ ] **Task: Implement Thread Hijacking**
    - [ ] Complete `thread_hijack` implementation in `modules/injection.rs`.
    - [ ] **Implement (Windows):** `SuspendThread`, `SetThreadContext`, `ResumeThread`.
- [ ] **Task: Implement Native Service Movement**
    - [ ] Complete `psexec` and `service_stop` logic in `modules/lateral.rs`.
    - [ ] **Implement (Windows):** Use SCM APIs (`OpenSCManager`, `CreateService`, `StartService`).
- [ ] **Task: Conductor - User Manual Verification 'Phase 3: Spectre Injection & Lateral Movement' (Protocol in workflow.md)**

## Phase 4: Spectre Credentials & Cleanup

- [ ] **Task: Implement LSASS Dumping (Windows)**
    - [ ] Replace `dump_lsass` stub in `modules/credentials.rs` with real logic.
    - [ ] **Implement:** Use `MiniDumpWriteDump` via dynamic resolution of `dbghelp.dll`.
- [ ] **Task: Final Zero-Stub Audit**
    - [ ] Global search for "TODO", "stub", "placeholder", "unimplemented", "mock", or hardcoded "fake" returns in `clients/wraith-redops/`.
    - [ ] Verify clean compilation for `x86_64-unknown-linux-gnu`.
    - [ ] Verify clean compilation for `x86_64-pc-windows-gnu`.
- [ ] **Task: Conductor - User Manual Verification 'Phase 4: Spectre Credentials & Cleanup' (Protocol in workflow.md)**
