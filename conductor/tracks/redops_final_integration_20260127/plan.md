# Implementation Plan: WRAITH-RedOps Final Integration & Gap Remediation

## Phase 1: P1 (Critical) & Foundational Fixes
- [x] Task: Implement DH Ratchet Protocol Message Exchange (P1-1) 65144e9
    - [x] Define `FRAME_REKEY_DH` in `c2/packet.rs`
    - [x] Update `Spectre` to send rekey request every 2 min / 1M packets
    - [x] Update `Team Server` to process rekey and return new public key
    - [x] Implement fallback to full Handshake Re-trigger
    - [x] Implement fallback to In-Band Piggybacking
- [x] Task: Finalize and Verify PowerShell Runner.dll (P1-2) cdc77d1
    - [x] Verify existing `Runner.dll` integrity and functionality
    - [x] Integrate C# source and build script for local compilation
    - [x] Implement runtime MSIL generation for fallback
- [x] Task: Fix CLR MetaHost CLSID (P2-5) 551937d
    - [x] Verify CLSID in `clr.rs` against official COM GUID
    - [x] Correct if necessary
- [x] Task: Modernize SMB Header Serialization (P2-7) 251d2b0
    - [x] Remove `unsafe` transmutes in `smb.rs`
    - [x] Implement explicit byte-by-byte serialization/deserialization
- [x] Task: Enhance Entropy Engine (P2-6 / 8.5) fdbc3f3
    - [x] Update `entropy.rs` to check RDRAND CF flag
    - [x] Integrate `/dev/urandom` for Linux entropy
    - [x] Implement hardware RNG support for ARM64
- [x] Task: Conductor - User Manual Verification 'Phase 1: P1 & Foundational Fixes' (Protocol in workflow.md)

## Phase 2: P2 (Medium) Issues & Integration
- [x] Task: Wire PowerShell Management IPC (P2-1) 223876a
    - [x] Implement Tauri commands for `SetPowerShellProfile` and `GetPowerShellProfile`
    - [x] Update `operator-client` frontend to use new commands
- [x] Task: Expand Console Command Parity (P2-2) d5abe3a
    - [x] Add `inject`, `bof`, `socks`, `screenshot`, `browser`, `net_scan`, and `service_stop` to `Console.tsx`
    - [x] Implement UI handlers for command arguments
- [x] Task: Implement Windows UDP Transport (P2-3) 270c4d3
    - [x] Implement WinSock2 logic in `spectre-implant` using hash-resolved APIs
    - [x] Ensure parity with HTTP transport for encryption and frame handling
- [x] Task: Secure Implant Registration (P2-4) e262b1d
    - [x] Update `Register` RPC on Team Server to decrypt registration data
    - [x] Validate `ephemeral_public` from `RegisterRequest`
- [x] Task: Complete VBA Macro Generation (P2-8) 51e8b48 / d5b7462
    - [x] Finalize VBA logic in `phishing.rs` for attachment-based initial access
    - [x] Implement VBA PE Loader (Reflective Injection) for memory execution
    - [x] Update UI for method selection (Drop vs Memory)
- [x] Task: Conductor - User Manual Verification 'Phase 2: P2 (Medium) Issues & Integration' (Protocol in workflow.md)

## Phase 3: P3 (Low) Issues
- [x] Task: Implement Browser Credential Decryption (P3-1) 1269496
    - [x] Integrate DPAPI decryption in `browser.rs`
- [ ] Task: Dynamic Linux .text Base Detection (P3-2)
    - [ ] Parse `/proc/self/maps` to find actual `.text` base in `obfuscation.rs`
- [ ] Task: Encrypt Mesh Discovery (P3-3)
    - [ ] Replace "WRAITH_MESH_HELLO" with encrypted discovery handshake
- [ ] Task: Implement Windows SMB Client (P3-4)
    - [ ] Build SMB2 client module for Windows using WinSock and Named Pipes
- [ ] Task: Secure Keylogger Implementation (P3-5)
    - [ ] Implement synchronization for the static key buffer in `collection.rs`
- [ ] Task: Refine SMB Tree Connect Logic (P3-6)
    - [ ] Fix the "assume success" logic in `smb.rs` response parsing
- [ ] Task: Final Dead Code Audit (P3-7)
    - [ ] Audit and remove all 10 identified `#[allow(dead_code)]` annotations
- [ ] Task: Conductor - User Manual Verification 'Phase 3: P3 (Low) Issues' (Protocol in workflow.md)

## Phase 4: Enhancements & Advanced Tradecraft
- [ ] Task: Implement Sleep Obfuscation Enhancements (8.1)
    - [ ] Implement Module Stomping
    - [ ] Implement Stack Spoofing
    - [ ] Implement CFG-aware sleep logic
- [ ] Task: Expand Indirect Syscalls (8.2)
    - [ ] Implement multiple gadget source redundancy
    - [ ] Implement egg hunting for `syscall; ret` in `ntdll`
    - [ ] Implement syscall number caching
- [ ] Task: Implement Malleable C2 Support (8.3)
    - [ ] Add support for configurable HTTP headers, URIs, and User Agents
    - [ ] Implement traffic jitter profiles
    - [ ] Implement Domain Fronting support
- [ ] Task: Integrate Post-Quantum Hybrid Key Exchange (8.4)
    - [ ] Implement ML-KEM (Kyber) hybrid with X25519
    - [ ] Update protocol frames to accommodate larger PQ keys
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Enhancements & Advanced Tradecraft' (Protocol in workflow.md)

## Phase 5: MITRE ATT&CK Completion
- [ ] Task: Complete Managed Windows Shell (T1059.003)
    - [ ] Finalize `modules/powershell.rs` integration
- [ ] Task: Integrate Thread Execution Hijack (T1055.003)
    - [ ] Verify full integration and Console UI availability
- [ ] Task: Implement Access Token Manipulation (T1134)
    - [ ] Create token duplication and impersonation module
- [ ] Task: Implement General Deobfuscation Utility (T1140)
    - [ ] Implement Base64, Hex, and XOR decoding in-implant
- [ ] Task: Implement DLL Side-Loading (T1574.002)
    - [ ] Create DLL proxying/side-loading templates
- [ ] Task: Finalize P2P Mesh Implementation (T1095)
    - [ ] Complete Non-Application Layer Protocol coverage
- [ ] Task: Implement Ingress Tool Transfer (T1105)
    - [ ] Build file upload capability to the implant
- [ ] Task: Conductor - User Manual Verification 'Phase 5: MITRE ATT&CK Completion' (Protocol in workflow.md)
