# Plan: RedOps Advanced Evasion & Features

## Phase 1: Core Evasion (Sleep & Syscalls)
- [ ] Task: Implement Indirect Syscalls
    - [ ] Update `syscalls.rs` to find `syscall` instruction in `ntdll.dll`.
    - [ ] Update `do_syscall` to jump to the found address.
- [ ] Task: Implement Sleep Obfuscation (Ekko)
    - [ ] Define `CreateTimerQueueTimer` and related APIs in `windows_definitions.rs`.
    - [ ] Implement ROP chain construction in `obfuscation.rs`.
    - [ ] Replace simple sleep with Ekko sleep.

## Phase 2: Runtime Protection Patching
- [ ] Task: Implement AMSI/ETW Patching
    - [ ] Create `modules/patch.rs`.
    - [ ] Implement `patch_amsi` and `patch_etw`.
    - [ ] Call patching before CLR load in `powershell.rs` / `clr.rs`.

## Phase 3: Capabilities & Operations
- [ ] Task: Implement KillDate & WorkingTime
    - [ ] Update `C2Config` in `c2/mod.rs`.
    - [ ] Add checks in `run_beacon_loop`.
- [ ] Task: Implement Screen Capture (T1113)
    - [ ] Create `modules/screenshot.rs`.
    - [ ] Implement GDI capture logic.
- [ ] Task: Implement Browser Credential Harvesting (T1555)
    - [ ] Create `modules/browser.rs`.
    - [ ] Implement file search for Chrome/Edge profiles.
    - [ ] Implement key file extraction (download for offline decryption).
