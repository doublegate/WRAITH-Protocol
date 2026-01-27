# Implementation Plan: Full Remediation for CodeQL Alert #7 (Spectre Implant)

This plan outlines the steps to secure sensitive data within the `no_std` Spectre implant, addressing the cleartext logging concerns identified by CodeQL.

## Phase 1: Security Primitives [checkpoint: 6876760] (`no_std` Infrastructure)

- [x] Task: Update Cargo.toml Dependencies (6876760)
    - [ ] Implement: Add `chacha20poly1305` and `zeroize` with `default-features = false`.
    - [ ] Implement: (Attempt) Uncomment `wraith-crypto` and check for `no_std` compatibility.
- [x] Task: Implement Tiered Entropy Source (6876760)
    - [ ] Implement: In `utils/entropy.rs`, implement `get_random_bytes()` using hardware instructions (`RDRAND`/`RDTSC`) with fallback to stack/address entropy.
- [x] Task: Implement SensitiveData - [ ] Task: Implement SensitiveData & SensitiveGuard SensitiveGuard (6876760)
    - [ ] Implement: Create `utils/sensitive.rs` with `SensitiveData` struct and `SensitiveGuard`.
    - [ ] Write Tests: Round-trip encryption/decryption test in `test_sensitive.rs`.
- [x] Task: Implement SecureBuffer (Memory Locking) (6876760)
    - [ ] Implement: Add `SecureBuffer` to `sensitive.rs` with `mlock` (Linux) and `VirtualLock` (Windows) support.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Security Primitives [checkpoint: 6876760]' (Protocol in workflow.md)

## Phase 2: Component Hardening [checkpoint: 25fffe5] (Core Modules)

- [x] Task: Secure Shell Execution (`shell.rs`) (25fffe5)
    - [ ] Implement: Change `Shell::exec()` to return `SensitiveData<Vec<u8>>`.
    - [ ] Implement: Add zeroization for command strings and intermediate output buffers.
- [x] Task: Encrypted LSASS Dumps (`credentials.rs`) (25fffe5)
    - [ ] Implement: Implement `MiniDumpWriteDump` callback to write dump to an in-memory `SecureBuffer`.
    - [ ] Implement: Encrypt the buffer and write the final artifact to disk.
- [x] Task: Hardened Keylogger - [ ] Task: Hardened Keylogger & Discovery Discovery (25fffe5)
    - [ ] Implement: Apply `Zeroize` to `KEY_BUFFER` in `collection.rs`.
    - [ ] Implement: Wrap `get_username()` and sensitive discovery output in `SensitiveData`.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Component Hardening [checkpoint: 25fffe5]' (Protocol in workflow.md)

## Phase 3: Global Audit Phase 3: Global Audit & Integration Integration [checkpoint: f7ffc27]

- [x] Task: Audit Lateral - [ ] Task: Audit Lateral & C2 Dispatch C2 Dispatch (f7ffc27)
    - [ ] Implement: Audit `lateral.rs` and `c2/mod.rs` to ensure all credential paths use `SensitiveData`.
    - [ ] Implement: Update `dispatch_tasks` to properly `.unlock()` sensitive results for Noise transport.
- [x] Task: Final Verification - [ ] Task: Final Verification & Formatting Formatting (f7ffc27)
    - [ ] Implement: Run `cargo fmt` and `cargo clippy`.
    - [ ] Write Tests: Integration test for encrypted dump workflow.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Global Audit Phase 3: Global Audit & Integration Integration [checkpoint: f7ffc27]' (Protocol in workflow.md)
