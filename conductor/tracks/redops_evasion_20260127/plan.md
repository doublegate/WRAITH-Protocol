# Plan: RedOps Advanced Evasion & Features

## Phase 1: Core Evasion (Sleep & Syscalls)
- [x] Task: Implement Indirect Syscalls [7cf2d953]
    - [x] Update `syscalls.rs` to find `syscall` instruction in `ntdll.dll`.
    - [x] Update `do_syscall` to jump to the found address.
- [x] Task: Implement Sleep Obfuscation (Ekko) [2d8b0771]
    - [x] Define `CreateTimerQueueTimer` and related APIs in `windows_definitions.rs`.
    - [x] Implement ROP chain construction in `obfuscation.rs`.
    - [x] Replace simple sleep with Ekko sleep.

## Phase 2: Runtime Protection Patching
- [x] Task: Implement AMSI/ETW Patching [47920ccd]
    - [x] Create `modules/patch.rs`.
    - [x] Implement `patch_amsi` and `patch_etw`.
    - [x] Call patching before CLR load in `powershell.rs` / `clr.rs`.

## Phase 3: Capabilities & Operations
- [x] Task: Implement KillDate & WorkingTime [505f2b51]
    - [x] Update `C2Config` in `c2/mod.rs`.
    - [x] Add checks in `run_beacon_loop`.
- [ ] Task: Implement Screen Capture (T1113)
    - [ ] Create `modules/screenshot.rs`.
    - [ ] Implement GDI capture logic.
- [ ] Task: Implement Browser Credential Harvesting (T1555)
    -   [ ] Create `modules/browser.rs`.
    -   [ ] Implement file search for Chrome/Edge profiles.
    -   [ ] Implement key file extraction (download for offline decryption).

## Phase 4: Integration & Lifecycle Management
- [ ] Task: Tauri Client Parity & Notification Support
    - [ ] Install and configure `tauri-plugin-log` in the Operator Client.
    - [ ] Install and configure `tauri-plugin-notification`.
    - [ ] Add notification triggers for high-priority events (check-ins, completions).
- [ ] Task: Version & Workspace Synchronization
    - [ ] Update `spectre-implant/Cargo.toml` version to `2.3.0` and edition to `2024`.
    - [ ] Update `team-server/Cargo.toml` version to `2.3.0`.
- [ ] Task: Conductor - Final Track Verification (All Phases)
