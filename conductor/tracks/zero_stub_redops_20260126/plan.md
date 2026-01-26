# Implementation Plan: WRAITH-RedOps Zero-Stub Completion

This plan enforces a strict "Zero Stub" policy for WRAITH-RedOps, replacing all placeholders with production-ready logic and updating the workflow guidelines to maintain this standard.

## Phase 1: Workflow Phase 1: Workflow & Guidelines Guidelines [checkpoint: ba7bf34]

- [~] **Task: Update Workflow Guidelines**
    - [ ] Modify `conductor/workflow.md` to add "Zero Stub" policy as Rule #1.
    - [ ] Explicitly forbid placeholders, skeleton implementations, or "coming soon" stubs in RedOps code.
- [ ] **Task: Conductor - User Manual Verification 'Phase 1: Workflow Phase 1: Workflow & Guidelines Guidelines [checkpoint: ba7bf34]' (Protocol in workflow.md)**

## Phase 2: Spectre Discovery & Collection

- [x] **Task: Implement Full Linux System Discovery** (ea9e04e)
    - [ ] Replace `sys_info` stub in `modules/discovery.rs` with real `uname` and `/proc` parsing.
    - [ ] **Write Tests:** Create unit tests to verify real system data is returned on Linux.
    - [ ] **Implement:** Use raw syscalls to fetch hostname, kernel version, and architecture.
- [x] **Task: Implement Full Network Scanner (Linux/Windows)** (ea9e04e)
    - [ ] Implement `net_scan` logic using raw socket syscalls (Linux) and Winsock (Windows).
    - [ ] **Write Tests (Linux):** Verify scanner can detect open ports on localhost.
    - [ ] **Implement:** Build a synchronous TCP connect scanner.
- [x] **Task: Complete Keylogger Implementation** (2f15995)
    - [ ] Ensure `keylogger` in `modules/collection.rs` is persistent and handles buffer management.
    - [ ] **Implement:** Use `SetWindowsHookEx` (Windows) or `/dev/input` (Linux - optional based on perms).
- [ ] **Task: Conductor - User Manual Verification 'Phase 2: Spectre Discovery & Collection' (Protocol in workflow.md)**

## Phase 3: Spectre Injection & Lateral Movement

- [x] **Task: Implement Robust Process Hollowing** (496e820)
    - [ ] Revisit `modules/injection.rs` to ensure full implementation of process hollowing.
    - [ ] **Implement (Windows):** `CreateProcess`, `NtUnmapViewOfSection`, `VirtualAllocEx`, `WriteProcessMemory`, `SetThreadContext`, `ResumeThread`.
    - [ ] **Implement (Linux):** `ptrace` based injection or similar.
- [x] **Task: Implement Thread Hijacking** (496e820)
    - [ ] Complete `thread_hijack` implementation in `modules/injection.rs`.
    - [ ] **Implement (Windows):** `SuspendThread`, `SetThreadContext`, `ResumeThread`.
- [x] **Task: Implement Native Service Movement** (d6d16e1)
    - [ ] Complete `psexec` and `service_stop` logic in `modules/lateral.rs`.
    - [ ] **Implement (Windows):** Use SCM APIs (`OpenSCManager`, `CreateService`, `StartService`).
- [ ] **Task: Conductor - User Manual Verification 'Phase 3: Spectre Injection & Lateral Movement' (Protocol in workflow.md)**

## Phase 4: Spectre Credentials Phase 4: Spectre Credentials & Cleanup Cleanup [checkpoint: bfe3a32]

- [x] **Task: Implement LSASS Dumping (Windows)** (496e820)
    - [ ] Replace `dump_lsass` stub in `modules/credentials.rs` with real logic.
    - [ ] **Implement:** Use `MiniDumpWriteDump` via dynamic resolution of `dbghelp.dll`.
- [x] **Task: Final Zero-Stub Audit** (a0045be)
    - [ ] Global search for "TODO", "stub", "placeholder", "unimplemented", "mock", or hardcoded "fake" returns in `clients/wraith-redops/`.
    - [ ] Verify clean compilation for `x86_64-unknown-linux-gnu`.
    - [ ] Verify clean compilation for `x86_64-pc-windows-gnu`.
- [ ] **Task: Conductor - User Manual Verification 'Phase 4: Spectre Credentials Phase 4: Spectre Credentials & Cleanup Cleanup [checkpoint: bfe3a32]' (Protocol in workflow.md)**
