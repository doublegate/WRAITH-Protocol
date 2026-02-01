# Specification: WRAITH-RedOps Final Alignment and Completion (v2.3.4)

## 1. Overview
The goal of this track is to methodically remediate all 12 identified gaps, 5 missing MITRE ATT&CK techniques, and various UI/UX deficiencies detailed in `GAP-ANALYSIS-v2.3.4.md`. This work will bring the WRAITH-RedOps platform to 100% completion, ensuring production-grade tradecraft, robust session security, and a polished operator experience.

## 2. Functional Requirements

### 2.1 Spectre Implant (Core & Tradecraft)
- **Signal-Style Double Ratchet (P1-1):** Implement a full Double Ratchet (DH + Symmetric) for session key management to ensure forward secrecy. Replace the current no-op rekeying logic.
- **PowerShell Runner Source-Build (P1-2):** Integrate C# source code for the PowerShell runner and implement a build step (using `dotnet`) to generate the `Runner.dll` artifact, replacing the current MZ stub.
- **Missing MITRE ATT&CK Techniques:** Implement full, advanced versions of:
    - **T1059.003:** Windows Command Shell (Managed execution).
    - **T1134:** Access Token Manipulation (Impersonation, Creation, and Delegation).
    - **T1140:** Deobfuscate/Decode (In-memory decoding of payloads).
    - **T1574.002:** DLL Side-Loading (Automated identification and exploitation of side-loading targets).
    - **T1105:** Ingress Tool Transfer (Multi-protocol support: HTTP/S, SMB, DNS).
- **P2/P3 Remediation:**
    - Verify CLR CLSID (P2-2).
    - Implement Browser DPAPI decryption (P3-1).
    - Dynamic Linux .text base address calculation (P3-2).
    - Obfuscate Mesh discovery signatures (P3-3).
    - Implement missing SMB client functionality for Windows (P3-4).
    - Upgrade compression from RLE to zlib/deflate (P3-5).
    - Replace `.unwrap()` calls in BOF parser with safe error handling (P3-6).

### 2.2 Team Server (Infrastructure & Protocol)
- **Double Ratchet Integration:** Update the `ProtocolHandler` to support the Double Ratchet handshake and per-message symmetric ratchet steps.
- **Kill Switch Safety (P2-3):** Replace `.expect()` calls in `operator.rs` with graceful error propagation to prevent runtime panics when env vars are missing.
- **Proper Nonce Management (P2-5):** Implement real nonce generation for response frames in `protocol.rs`, replacing the current static placeholders.

### 2.3 Operator Client (UX & Frontend)
- **Full Console Coverage (P2-1):** Map the remaining 4 implant commands (`compress`, `exfil_dns`, `wipe`, `hijack`) in `Console.tsx`.
- **Resource Management:** Implement "Delete" functionality for Listeners and Attack Chains.
- **UI Polish:**
    - Update version string to `v2.3.4` globally.
    - Implement Bulk Implant operations (e.g., multi-beacon tasks).
    - Add Dark/Light theme toggle support.

## 3. Non-Functional Requirements
- **Implant Constraints:** Maintain strict `#![no_std]` compliance for `spectre-implant`.
- **Code Quality:** Adhere to "Zero Warnings" policy across all crates.
- **Security:** Ensure all cryptographic implementations use constant-time primitives and zeroize sensitive memory.
- **Documentation:** Full code comments for all new modules, explaining implementation particulars and tradecraft.

## 4. Acceptance Criteria
- [ ] 40/40 planned MITRE ATT&CK techniques fully implemented and functional.
- [ ] All 12 findings from GAP-ANALYSIS-v2.3.4 marked as "RESOLVED".
- [ ] Signal-style Double Ratchet verified via successful long-running session rekeying.
- [ ] PowerShell Runner successfully executes managed code via the new `Runner.dll` built from source.
- [ ] UI reflects 100% feature parity with the backend and displays the correct version string.
- [ ] All existing and new tests pass in the workspace.
