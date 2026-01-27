# Track: RedOps Advanced Evasion & Features

## Goal
Implement advanced evasion techniques and missing MITRE ATT&CK capabilities to bring WRAITH-RedOps to production-grade tradecraft standards.

## Scope
1.  **Sleep Obfuscation (Priority: High):** Implement ROP-chain based sleep (Ekko/Foliage) to encrypt memory and avoid direct API calls during sleep.
2.  **Indirect Syscalls (Priority: Medium):** Execute syscalls by jumping to `ntdll.dll` instructions to avoid direct syscall usage.
3.  **AMSI/ETW Patching (Priority: Medium):** Patch memory to disable AMSI and ETW logging before loading CLR.
4.  **Operational Controls (Priority: Low):** Implement autonomous KillDate and WorkingTime constraints.
5.  **MITRE Capabilities (Priority: P2):**
    -   **T1113:** Screen Capture
    -   **T1555:** Browser/Keychain Credential Access
6.  **Integration & Lifecycle Management (Phase 4):**
    -   **Tauri Client Parity:** Upgrade operator client to Tauri 2.x standards, including `tauri-plugin-log` and `tauri-plugin-notification`.
    -   **Version Synchronization:** Align `spectre-implant` versioning with the project core (v2.3.0).
    -   **Workspace Integration:** Address CI/CD gaps caused by workspace exclusion.

## Implementation Details

### Sleep Obfuscation
-   Use `CreateTimerQueueTimer` to queue a ROP chain.
-   ROP chain: `VirtualProtect` (RW) -> `Encrypt` -> `Sleep` -> `Decrypt` -> `VirtualProtect` (RX).
-   Requires `CONTEXT` manipulation and careful stack handling.

### Indirect Syscalls
-   Resolve SSN as before.
-   Instead of `syscall` instruction inline, find `syscall` instruction address in `ntdll.dll`.
-   Jump to that address.

### AMSI/ETW Patching
-   Resolve `AmsiScanBuffer` (amsi.dll) and `EtwEventWrite` (ntdll.dll).
-   Change protection to RWX.
-   Patch with `ret` (or similar).
-   Restore protection.

### KillDate / WorkingTime
-   Update `C2Config` struct.
-   Check in `run_beacon_loop`.

### Screen Capture
-   Use GDI APIs (`CreateCompatibleDC`, `BitBlt`, etc.) to capture screen to bitmap.
-   Convert to PNG/JPEG if possible, or raw BMP.

### Browser Credentials
-   Search for `Login Data` files (Chrome/Edge).
-   Use DPAPI (`CryptUnprotectData`) to decrypt the key (if master key extraction is feasible in no_std, otherwise just download the files).
-   Implement key file extraction (download for offline decryption).

### Tauri & Lifecycle
-   Update `tauri.conf.json` to include missing plugins.
-   Implement desktop notifications for implant check-ins and command completions.
-   Synchronize `spectre-implant/Cargo.toml` version to `2.3.0`.
-   Add explicit linting/testing commands for RedOps components in `xtask` or CI.