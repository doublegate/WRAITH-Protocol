# Track Specification: WRAITH-RedOps Gap Remediation (v2.2.5)

## 1. Overview
This track focuses on the methodical remediation of all remaining gaps, deficiencies, and misalignments in the WRAITH-RedOps platform as identified in the `GAP-ANALYSIS-v2.2.5.md` document. The goal is to bring the Team Server, Operator Client, and Spectre Implant into full alignment with the intended architectural specifications and tradecraft requirements.

## 2. Methodology
1.  **Priority-Based Implementation:** Gaps will be addressed in phases based on their severity (P1 -> P2 -> P3).
2.  **Platform-Specific Validation:**
    *   **Linux:** Automated unit/integration tests plus standalone verification harnesses (small Rust binaries) to prove syscall success on the local development host.
    *   **Windows:** Verification of successful cross-compilation (`x86_64-pc-windows-gnu`) and detailed manual verification checklists for the operator to run on a target Windows system.
3.  **Zero Stub Policy:** No placeholders or "todo" comments. Every remediated feature must be fully functional.

## 3. Remediated Findings (Source: GAP-ANALYSIS-v2.2.5.md)

### P1 - High Priority (Core Functionality)
*   **Key Ratcheting (Team Server):** Implement Noise DH key ratcheting per spec (every 2min/1M packets).
*   **PowerShell Runner (Spectre):** Replace minimal MZ placeholder with a real compiled .NET PowerShell runner assembly.
*   **Attack Chain IPC Bridge (Operator Client):** Register the 4 missing attack chain commands in the Tauri backend (`lib.rs`) and wire them to the gRPC client.

### P2 - Medium Priority (Platform Completeness)
*   **DNS Multi-Label Encoding (Team Server):** Support multi-label chunked payload encoding in the DNS listener.
*   **Heap Address Discovery (Spectre):** Implement runtime heap discovery via `/proc/self/maps` (Linux) and dynamic API calls (Windows) instead of hardcoded addresses.
*   **Noise Handshake Error Handling (Spectre):** Replace `.unwrap()`/`.expect()` calls in the handshake sequence with robust error handling.
*   **CLR GUID Correction (Spectre):** Use correct CLSID for `CLRRuntimeHost` in the CLR module.
*   **LLVM Obfuscation Flags (Builder):** Implement actual `RUSTFLAGS` for LLVM-level obfuscation passes in the builder pipeline.
*   **Scheduled Task Native COM (Spectre):** Implement full COM-based `ITaskService` vtable for scheduled task persistence (remove shell fallback).
*   **VBA Shellcode Runner (Builder):** Implement the `CreateThread(VirtualAlloc(code))` runner in the phishing macro generator.
*   **AttackChainEditor Wiring (Operator Client):** Wire the visual editor's `handleExecute` and "Save" buttons to the real IPC commands (removing simulation logic).

### P3 - Low Priority (Enhancements)
*   **P2P Mesh C2 (Team Server/Implant):** Implement initial peer-to-peer beacon routing.
*   **APT Playbooks (Team Server):** Implement automated technique sequences.
*   **SMB2 Full Protocol (All):** Move from simplified length-prefix framing to a more robust SMB2-style header implementation.
*   **DNS TXT Record Formatting (Spectre):** Fix TXT record wrapping to ensure valid RDATA parsing.
*   **Settings UI (Operator Client):** Add a UI for configuring the Team Server address and other preferences.
*   **Keylogger Persistence (Spectre):** Move from single-poll to a persistent keylogger with configurable interval.
*   **Process Hollowing ImageBase (Spectre):** Query PEB for the actual ImageBase instead of assuming `0x400000`.

## 4. Acceptance Criteria
*   All findings in the Priority Matrix (P1, P2, P3) marked as "RESOLVED".
*   Successful compilation for both Linux and Windows targets.
*   Linux syscall features verified via standalone harnesses on the host.
*   Operator Client visual editor functional and communicating with the backend.
*   Audit log entries verify all new activities are signed and recorded.
*   Zero `.unwrap()` calls remaining in production-critical paths.
