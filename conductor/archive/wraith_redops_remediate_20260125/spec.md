# Specification: Remediate WRAITH-RedOps Implementation (Gap Analysis v3.0.0)

## Overview
This track focuses on the methodical remediation of all findings in the `GAP-ANALYSIS-v2.2.5.md` document, specifically Version 3.0.0 (Deep Implementation Audit). The goal is to correct every gap, stub, placeholder, and misalignment identified, bringing the Team Server, Operator Client, and Spectre Implant to a fully functional, production-ready state.

## Functional Requirements

### 1. Team Server Remediation
*   **1.1 UDP Transport (Critical):**
    *   **Gap:** HTTP listener exists, but UDP transport is missing (Gap 1.1).
    *   **Requirement:** Implement a full UDP listener in `listeners/udp.rs` that supports the WRAITH protocol.
    *   **Requirement:** Port the Noise handshake logic to work over UDP.
*   **1.2 Kill Switch (Critical):**
    *   **Gap:** Kill switch missing (Gap 1.2).
    *   **Requirement:** Implement the `team-server/src/kill_switch.rs` module.
    *   **Requirement:** Create a UDP broadcast mechanism for the halt signal with <1ms response time.
*   **1.3 Governance Enforcement:**
    *   **Gap:** Domain validation missing, Audit logging unsigned (Gap 1.3).
    *   **Requirement:** Implement Domain Validation in `governance.rs`.
    *   **Requirement:** Implement signed Audit Logging in `database/mod.rs`.
*   **1.4 Task Delivery:**
    *   **Gap:** `listeners/http.rs` (line 223) returns empty tasks (Gap 1.4).
    *   **Requirement:** Fix `listeners/http.rs` to query the database for pending commands instead of returning empty vectors.
    *   **Requirement:** Serialize real commands into the response payload.
*   **1.5 Real-time Events:**
    *   **Gap:** `stream_events` returns empty stream (Gap 1.5).
    *   **Requirement:** Connect the broadcast channel in `main.rs` to actual event sources so `stream_events` in `services/operator.rs` (lines 413-419) works.
*   **1.6 Builder Pipeline:**
    *   **Gap:** `builder` module missing (Gap 1.6).
    *   **Requirement:** Implement the `team-server/src/builder/` module for dynamic implant compilation.
    *   **Requirement:** Implement template selection, config patching, and LLVM obfuscation.
*   **1.7 Listener Stubs:**
    *   **Gap:** `listeners/dns.rs` and `listeners/smb.rs` are log-only stubs.
    *   **Requirement:** Flesh out `listeners/dns.rs` with functional DNS tunneling (using `trust-dns`).
    *   **Requirement:** Flesh out `listeners/smb.rs` with functional SMB named pipe logic.

### 2. Operator Client Remediation
*   **2.1 Interactive Console:**
    *   **Gap:** Placeholder button only (Gap 2.1).
    *   **Requirement:** Implement the xterm.js integration (currently a placeholder button in `App.tsx` line 209).
    *   **Requirement:** Connect command history and tab completion.
*   **2.2 Graph Visualization:**
    *   **Gap:** Missing graph component (Gap 2.2).
    *   **Requirement:** Implement the D3.js beacon topology graph.
*   **2.3 Campaign Creation:**
    *   **Gap:** Backend exists, UI missing (Gap 2.3).
    *   **Requirement:** Connect the `create_campaign` IPC to a UI wizard.
*   **2.4 Missing IPC Commands:**
    *   **Gap:** Missing IPC commands (Gap 2.4).
    *   **Requirement:** Implement `download_artifact`, `kill_implant`, `update_campaign`, and listener controls in `src-tauri/src/lib.rs`.

### 3. Spectre Implant Remediation
*   **3.1 WRAITH C2 Integration:**
    *   **Gap:** HTTP transport only, no client-side Noise (Gap 3.1).
    *   **Requirement:** Implement client-side Noise_XX handshake in `c2/mod.rs`.
    *   **Requirement:** Implement Elligator2 and Double Ratchet.
*   **3.2 Sleep Mask:**
    *   **Gap:** Simple XOR only (Gap 3.2).
    *   **Requirement:** Implement ROP chain for memory encryption (encrypt .text and .data).
*   **3.3 Indirect Syscalls:**
    *   **Gap:** Windows SSN resolution missing (Gap 3.3).
    *   **Requirement:** Implement Windows SSN resolution (Hell's Gate/Halo's Gate) in `utils/syscalls.rs`.
*   **3.4 API Resolution:**
    *   **Gap:** `GetProcAddress` logic missing (Gap 3.4).
    *   **Requirement:** Connect `GetProcAddress` resolution in `utils/api_resolver.rs`.
*   **3.5 Post-Exploitation Stubs:**
    *   **Gap:** All modules are stubs (Gap 3.5, 3.6, 3.7).
    *   **Requirement:** Implement `modules/bof_loader.rs` (COFF parsing/execution).
    *   **Requirement:** Implement `modules/injection.rs` (Reflective DLL, Process Hollowing, Thread Hijacking).
    *   **Requirement:** Implement `modules/socks.rs` (SOCKS4a/5 proxy).
*   **3.6 Task Execution:**
    *   **Gap:** Loop stub only (Gap 3.8).
    *   **Requirement:** Implement task parsing and command dispatch in `c2/mod.rs`.

## Technical Debt & Code Quality
*   **Hardcoded Values:** Externalize all hardcoded IPs, ports, and secrets identified in the audit (e.g., `main.rs`, `utils.rs`, `lib.rs`).
*   **Error Handling:** Replace all production `.unwrap()` calls with proper error handling.
*   **Comments:** Replace all placeholder comments ("In a real implementation...", "TODO") with actual code.

## Acceptance Criteria
- All 30+ missing/stub features identified in the Gap Analysis are fully implemented.
- No "TODO", "FIXME", or "In a real implementation" comments remain in critical paths.
- All hardcoded configuration values are moved to environment variables or config files.
- The `cargo test` suite passes with meaningful tests for new functionality.
- The Operator Client UI is fully functional and connected to the backend.
