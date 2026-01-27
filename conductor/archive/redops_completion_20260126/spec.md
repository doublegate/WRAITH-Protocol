# Track Specification: WRAITH-RedOps Implementation Completion

## 1. Overview
This track aims to methodically remediate all findings and complete the implementation of the WRAITH-RedOps adversary emulation platform. It follows the "Revised Timeline Estimate" (7 Sprints) defined in the detailed Gap Analysis (v4.1.0) to resolve critical security gaps, core functionality bugs, and missing tradecraft features.

## 2. Scope
The work is strictly defined by the **Revised Timeline Estimate** in `docs/clients/wraith-redops/GAP-ANALYSIS-v2.2.5.md`.

### Sprint 1: P0 Critical Security
*   **Focus:** Critical security vulnerability in gRPC authentication.
*   **Tasks:**
    *   Fix `gRPC Auth Passthrough`: Modify the interceptor to explicitly reject requests without a valid Authorization header (currently `None => Ok` allows bypass).

### Sprint 2: P1 Core Bugs & Gaps
*   **Focus:** Blocking bugs and empty placeholders in the Implant.
*   **Tasks:**
    *   **Fix CONTEXT Struct:** Correct the structural bug in `windows_definitions.rs` where fields are orphaned outside the struct.
    *   **Kill Signal Config:** Externalize the hardcoded port (6667) and secret (`b"secret"`) to environment variables or config.
    *   **PowerShell Runner:** Replace the `MZ_PLACEHOLDER` with a proper mechanism (or embedded resource) for the .NET runner assembly.
    *   **BeaconDataParse:** Implement the missing argument parsing logic for BOF compatibility.

### Sprint 3: P1 C2 Expansion
*   **Focus:** Core Command & Control capabilities.
*   **Tasks:**
    *   **Task Dispatch:** Implement dispatch logic for missing task types: `inject`, `bof`, `socks` (currently only `kill` and `shell` work).
    *   **SOCKS TCP Relay:** Implement actual TCP connection relaying (currently simulated).
    *   **Key Ratcheting:** Implement Noise protocol key ratcheting (Diffie-Hellman update) every 2 minutes or 1M packets.

### Sprint 4: P1 Dynamic Management + Beacon Data
*   **Focus:** Operator control and visibility.
*   **Tasks:**
    *   **Dynamic Listeners:** Implement logic to actually spawn/abort Tokio tasks when `start_listener` / `stop_listener` RPCs are called (currently only updates DB status).
    *   **Beacon Data:** Populate beacon metadata from actual system information (currently static JSON).

### Sprint 5: P2 Platform & Stubs
*   **Focus:** Platform compatibility and missing modules.
*   **Tasks:**
    *   **Linux Implementation:** Implement `process_vm_writev` / `ptrace` logic for injection, discovery, and lateral movement on Linux (currently returns stubs).
    *   **Credential Dumping:** Implement `MiniDumpWriteDump` or LSASS memory parsing logic (currently a stub).
    *   **Network Scanner:** Implement actual TCP connect scan logic.
    *   **XOR Key:** Randomize the sleep mask XOR key (currently hardcoded `0xAA`).

### Sprint 6: P2 Completeness
*   **Focus:** Robustness and evasion.
*   **Tasks:**
    *   **DNS Encoding:** Implement multi-label chunked payload encoding for DNS TXT records.
    *   **Artifact Encryption:** Encrypt artifacts at rest in the database (currently plaintext).
    *   **Obfuscation:** Implement RUSTFLAGS or other build-time obfuscation passes.
    *   **Config:** Externalize hardcoded listener ports (8080, 9999, etc.) in `main.rs`.
    *   **Native Persistence:** Re-implement persistence to use native APIs instead of shelling out to `cmd.exe`.

### Sprint 7: P3 Advanced Features
*   **Focus:** High-end tradecraft.
*   **Tasks:**
    *   **Sleep Mask:** Implement ROP-based memory encryption for `.text` section during sleep.
    *   **P2P Mesh:** Implement peer-to-peer beacon routing (SMB/TCP).
    *   **APT Playbooks:** Implement automated technique sequencing.
    *   **SMB2 Protocol:** Upgrade from simple framing to full SMB2 protocol headers.
    *   **Keylogger:** Implement continuous monitoring and full virtual key mapping.

## 3. Development Constraints
*   **Target Environments:**
    *   **Windows:** Code will be implemented and checked for compilation only (`cargo check --target x86_64-pc-windows-gnu`). Runtime verification is OUT OF SCOPE.
    *   **Linux:** Code will be implemented, checked for compilation, and verified at runtime since the development environment is Linux.
    *   Use `#[cfg(target_os = "windows")]` and `#[cfg(not(target_os = "windows"))]` guards where appropriate.
*   **Verification:**
    *   **Team Server / Client:** Must be verified with unit/integration tests running on Linux.
    *   **Implant (Cross-Platform):** Logic that is platform-agnostic must be tested.
    *   **Implant (Linux-Specific):** Must be verified via unit tests or manual verification on the host system.

## 4. Acceptance Criteria
*   All items listed in the Gap Analysis v4.1.0 "Revised Timeline Estimate" are implemented or fixed.
*   The Team Server allows **zero** unauthenticated gRPC requests.
*   The `CONTEXT` struct definition matches the Windows API requirement.
*   Code compiles without errors for both `x86_64-unknown-linux-gnu` and `x86_64-pc-windows-gnu`.
