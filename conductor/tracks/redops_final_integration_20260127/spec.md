# Track Specification: WRAITH-RedOps Final Integration & Gap Remediation

## Overview
This track executes the comprehensive finalization of the WRAITH-RedOps advanced client. It encompasses the methodical remediation of all 17 findings from Gap Analysis v2.3.0, the implementation of 5 major tradecraft enhancements, and the completion of the 7 remaining MITRE ATT&CK techniques. The goal is a production-grade, highly evasive, and fully featured adversary emulation platform.

## Functional Requirements

### Phase 1: P1 (Critical) & Foundational Fixes
- **P1-1: Key Ratcheting (Forward Secrecy):**
  - Implement tiered fallback strategy: Dedicated Protocol Message -> Handshake Re-trigger -> In-Band Piggybacking.
  - Enforce rekey interval (2 mins / 1M packets).
- **P1-2: PowerShell Runner DLL:**
  - Verify and finalize `Runner.dll`.
  - Implement tiered fallback: Source/Compilation -> Embedded Binary -> Runtime MSIL.
- **P2-5: CLR CLSID Verification:** Verify and correct the CLR MetaHost CLSID in `clr.rs`.
- **P2-7: SMB Header Serialization:** Replace `unsafe` transmutes with explicit byte serialization.
- **P2-6 / Enhancement 8.5: Improved Entropy:**
  - Check RDRAND CF flag.
  - Use `/dev/urandom` on Linux.
  - Implement hardware RNG support for ARM64.

### Phase 2: P2 (Medium) Issues & Integration
- **P2-1: PowerShell IPC Gap:** Wire `SetPowerShellProfile` and `GetPowerShellProfile` RPCs to Tauri IPC.
- **P2-2: Console Command Parity:** Add `inject`, `bof`, `socks`, `screenshot`, `browser`, `net_scan`, and `service_stop` to `Console.tsx`.
- **P2-3: Windows UDP Transport:** Implement functional WinSock2 UDP transport in `no_std` implant with hash-resolved APIs and full protocol parity.
- **P2-4: Implant Registration:** Enhance `Register` RPC to decrypt registration data and validate ephemeral keys.
- **P2-8: VBA Macro Generator:** Complete VBA macro generation logic in `phishing.rs`.

### Phase 3: P3 (Low) Issues
- **P3-1: Browser Decryption:** Implement DPAPI-protected credential decryption in `browser.rs` to recover actual credentials.
- **P3-2: Linux .text Detection:** Dynamic `.text` base detection via `/proc/self/maps`.
- **P3-3: Encrypted Mesh Discovery:** Replace plaintext broadcast with encrypted/obfuscated handshake.
- **P3-4: SMB Client (Windows):** Implement SMB2 client module for Windows (WinSock/Named Pipe).
- **P3-5: Keylogger Thread Safety:** Implement synchronization for the key buffer.
- **P3-6: SMB Parse Logic:** Fix "assume success" logic in tree connect.
- **P3-7: Dead Code Removal:** Audit and remove all 10 identified `#[allow(dead_code)]` annotations.

## Acceptance Criteria
- [ ] All 17 Gap Analysis findings (P1-P3) are resolved according to the spec.
- [ ] All 5 Enhancement Recommendations are implemented.
- [ ] All 7 remaining MITRE ATT&CK techniques are implemented and verified.
- [ ] Spectre Implant remains `no_std` and compiles for both Windows and Linux.
- [ ] Operator Client UI provides full command parity with the implant.
- [ ] All automated tests (including new rekeying and transport tests) pass.
- [ ] Zero clippy warnings across the `wraith-redops` crates.

## Out of Scope
- Future protocol enhancements (Post-Quantum, etc.) not specifically mentioned in the v2.3.0 Gap Analysis.
