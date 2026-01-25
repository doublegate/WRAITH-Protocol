# Specification: WRAITH-RedOps Remediation (Gap Analysis v2.2.5)

## 1. Overview
This track focuses on the comprehensive remediation of the WRAITH-RedOps capability, strictly following the findings and guidance detailed in `docs/clients/wraith-redops/GAP-ANALYSIS-v2.2.5.md` (Version 3.2.0). The goal is to bring the Team Server, Spectre Implant, and Operator Client to a fully functional, production-ready state by addressing all identified gaps, stubs, and technical debt.

## 2. Execution Strategy
- **Order:** Strict adherence to the Gap Analysis document structure (Section 1 -> Section 2 -> Section 3).
- **Completeness:** All items, including those marked "Deferred" or "Low Priority" in the analysis, are in scope and must be implemented.
- **Directives:** "Do not stub", "Do not skip", "Superset Principle" (preserve existing functionality while adding new).

## 3. Functional Requirements

### Phase 1: Team Server (Section 1)
- **1.1 Operator Service:** Implement proper `operator_id` extraction from gRPC metadata.
- **1.2 DNS Listener:** Replace stub with full DNS tunneling implementation (TXT record exfil, A/AAAA beaconing).
- **1.3 SMB Listener:** Replace stub with full SMB named pipe server implementation.
- **1.4 Implant Service:** Implement command/payload decryption (remove "In real impl" placeholders) and return actual compiled binaries.
- **1.5 HTTP Listener:** Connect to DB for tasks (remove `vec![]` stub), implement Frame wrapping, and fix `unwrap()` calls.
- **1.6 - 1.8 Configuration & Security:** Ensure all hardcoded values (DB URL, gRPC Addr, JWT Secret) are externalized (Env Vars) and `unwrap()` calls are handled safely.
- **1.9 Builder:** Implement the full implant build pipeline (beyond basic byte patching).

### Phase 2: Spectre Implant (Section 2)
- **2.1 Injection Modules:** Implement full `reflective_inject`, `process_hollowing`, and `thread_hijack` (replace `Ok(())` stubs).
- **2.2 BOF Loader:** Implement full COFF parsing, relocation, and execution for Beacon Object Files.
- **2.3 SOCKS Proxy:** Implement full SOCKS4a/5 authentication and request handling.
- **2.4 C2 Core:** Fix hardcoded IPs, implement PTY shell execution, and handle Noise protocol errors without panics.
- **2.5 Syscalls:** Implement Halo's Gate SSN resolution.
- **2.6 - 2.9 General:** Fix hardcoded heap addresses and complete the Shell module.

### Phase 3: Operator Client (Section 3)
- **3.1 Dashboard:** Implement full metrics retrieval (Server -> Client) to populate the dashboard (Campaigns, Beacons, Listeners, Artifacts).
- **3.2 Console:** Ensure command history and local commands are fully hooked up to the backend.
- **3.3 Network Graph:** Ensure the visual graph reflects real-time topology from the backend.
- **3.4 IPC & Safety:** Implement actual IPC data retrieval (remove `vec![]` returns) and ensure all unsafe blocks are necessary/safe.

## 4. Non-Functional Requirements
- **Test Coverage:** Implement unit tests for all remediated logic. Aim to satisfy the "Test Cases from Specification" (TC-001 to TC-010) where feasible.
- **Code Style:** Strict adherence to Rust idioms (no `unwrap` in production, proper error propagation).
- **Architecture:** Maintain `no_std` compliance for the Implant.

## 5. Acceptance Criteria
- All 23 specific findings in the "Detailed Findings by Component" section of the Gap Analysis are resolved.
- No functional stubs (e.g., functions returning `Ok(())` without logic) remain in the codebase.
- "Gap Analysis" re-run would yield 100% completion.
