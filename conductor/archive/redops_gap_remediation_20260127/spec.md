# Track Specification: WRAITH-RedOps Gap Remediation (v2.2.5)

## 1. Overview
This track executes a methodical, consecutive remediation of all findings identified in `docs/clients/wraith-redops/GAP-ANALYSIS-v2.2.5.md`. The goal is to transform WRAITH-RedOps from a ~91% complete prototype into a 100% complete, production-ready adversary emulation platform. Remediation is prioritized by severity (P1 -> P2 -> P3).

## 2. Functional Requirements

### 2.1 Phase 1: High Priority (P1) - Core Stability & Crypto
*   **[NEW-17] SMB2 Compilation Fix:** Rename `reserved` to `process_id` and fix `credit_request` usage in `Smb2Header` to ensure Team Server compiles.
*   **[P1 #12] Advanced Key Ratcheting:** Implement a full **Double Ratchet** protocol (KDF-chain based) for Noise sessions, triggered every 2 minutes or 1 million packets.
*   **[NEW-3] PowerShell Runner Finalization:** Embed a real, minimal functional .NET PowerShell runner assembly in `spectre-implant` to replace the MZ placeholder.

### 2.2 Phase 2: Medium Priority (P2) - Platform Completeness
*   **[NEW-18/19] IPC Bridge Completion:** wire all 9 missing gRPC methods into the Tauri IPC bridge (`RefreshToken`, `GetCampaign`, `GetImplant`, `CancelCommand`, `StreamEvents`, `GenerateImplant`, `ListPlaybooks`, `InstantiatePlaybook`).
*   **[P2 #19] Runtime Heap Discovery:** Implement dynamic heap discovery in `spectre-implant` using `/proc/self/maps` (Linux) and `GetProcessHeap` (Windows) to replace hardcoded ranges.
*   **[NEW-8] Native COM Persistence:** Implement full `ITaskService` COM vtables in `persistence.rs` to remove the `schtasks.exe` shell fallback.
*   **[NEW-10] Phishing Runner:** Implement a functional VBA shellcode runner (VirtualAlloc/CreateThread) in the phishing macro generator.
*   **[Misc P2]** Fix CLR GUIDs, LLVM obfuscation RUSTFLAGS integration, and `.unwrap()` cleanup in Noise handshakes.

### 2.3 Phase 3: Low Priority (P3) - Advanced Features & Quality
*   **[P3 #24] Structured P2P Mesh C2:** Implement a robust, DHT-based or Tree-based peer-to-peer routing system allowing beacons to relay C2 traffic through other beacons.
*   **[NEW-12/13] Tradecraft Polish:** Implement persistent keylogger monitoring and dynamic ImageBase querying for process hollowing.
*   **[P3 #28] UI Polish:** Add a Settings/Preferences UI to the Operator Client for server address configuration.
*   **[NEW-20] Test Expansion:** Significant expansion of unit and integration tests across all components to reach >20% coverage.

## 3. Methodology & Workflow
1.  **Consecutive Execution:** Work through the `GAP-ANALYSIS-v2.2.5.md` findings one-by-one as defined in the plan.
2.  **Zero Stub Policy:** No placeholders or "future development" stubs allowed.
3.  **Cross-Platform Parity:** Ensure Windows and Linux implementations reach parity where possible.

## 4. Acceptance Criteria
*   Zero findings remain in `GAP-ANALYSIS-v2.2.5.md`.
*   All 30 gRPC methods are accessible via the Operator Client UI.
*   P2P Mesh routing successfully delivers tasks to non-directly connected beacons.
*   Double Ratchet verified by monitoring session key rotation at 2-minute intervals.
*   Spectre Implant remains `no_std` and compiles for both Windows and Linux targets.
