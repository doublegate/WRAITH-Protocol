# Specification: WRAITH-RedOps Final Remediation & Completion (v4.0.0 Gap Analysis & MITRE ATT&CK Integration)

## 1. Overview
This track executes the final, comprehensive remediation of the WRAITH-RedOps platform based on the corrected Gap Analysis v4.0.0. The objective is to achieve **100% feature completion** and **production readiness** by eliminating all remaining stubs, partial implementations, hardcoded values, and insecure fallbacks. Additionally, it includes a rigorous implementation of offensive tradecraft mapped to the MITRE ATT&CK framework.

## 2. Scope & Execution Strategy
- **Source of Truth:**
    1. `docs/clients/wraith-redops/GAP-ANALYSIS-v2.2.5.md` (v4.0.0).
    2. Provided MITRE ATT&CK Offensive Techniques table (Phase 5).
- **Execution Order:** Strict adherence to the priority phases (P0 -> P1 -> P2 -> P3 -> P5).
- **Mandate:** "FULLY IMPLEMENT EVERYTHING." Any feature marked as "Partial," "Substantially Implemented," "Stub," or "Missing" must be brought to "FULLY IMPLEMENTED" status. All `TODO`, `FIXME`, and placeholder comments must be resolved with actual logic. No "future work" placeholders allowed.

## 3. Detailed Requirements (By Phase)

### Phase 1: P0 - Critical Safety & Security
- **Team Server:**
    - Remove hardcoded cryptographic key fallbacks (Database Master Key, HMAC Key). Enforce environment variables.
    - Remove hardcoded KillSwitch key seed. Implement secure loading or generation.
    - Implement gRPC channel authentication (mTLS or Token Interceptor) to prevent unauthorized access.
    - Implement proper Ed25519 signature verification for Operator authentication (replace weak "non-empty" check).

### Phase 2: P1 - High Priority Core Functionality
- **Spectre Implant (Windows):**
    - **Thread Hijack:** Implement full thread enumeration (Toolhelp32/NtQuerySystemInformation), context manipulation, and execution redirection.
    - **Process Hollowing:** Implement proper `NtUnmapViewOfSection` and PE re-mapping logic (replace fallback to reflective inject).
    - **BOF Loader:** Implement external symbol resolution (IAT) and BIF (Beacon Internal Functions) like `BeaconPrintf` (output capture) and `BeaconDataParse`.
    - **Task Dispatch:** Implement dispatch logic for ALL task types (inject, bof, socks, etc.), not just "kill/shell".
    - **SOCKS Proxy:** Implement actual TCP relay logic (connecting to target host/port) to replace simulated success.
- **Team Server:**
    - **Key Ratcheting:** Implement Noise protocol key ratcheting (every 2min/1M packets) in `protocol.rs` / `session.rs`.
    - **Dynamic Listeners:** Implement logic to actually spawn/abort listener tasks when `start_listener` / `stop_listener` are called (currently DB-only updates).

### Phase 3: P2 - Medium Priority Completeness
- **Spectre Implant (Linux):**
    - Implement Injection methods (Reflective, Hollowing, Thread Hijack) using `ptrace` / `process_vm_writev`.
    - Implement Halo's Gate SSN resolution (currently a stub).
    - Implement Heap Address Discovery (replace hardcoded addresses).
- **Team Server:**
    - Implement DNS multi-label encoding/decoding for larger payloads.
    - Implement Artifact Encryption at rest (currently plaintext).
    - Externalize hardcoded listener ports (8080, 9999, 5454, 4445) to config/env.
- **General:**
    - Eliminate `unwrap()` calls in production paths (replace with proper error handling).

### Phase 4: P3 - Low Priority & Enhancements (Mandatory Completion)
- **Spectre Implant:**
    - Implement Sleep Mask (ROP chain for .text section encryption).
    - Fix DNS TXT record formatting (remove quotes/ensure RDATA compliance).
- **Team Server:**
    - Implement P2P Mesh C2 routing logic.
    - Implement APT Playbook automation engine.
    - Upgrade SMB listener to full SMB2 protocol headers (replace simplified framing).
- **Operator Client:**
    - Implement Settings UI for server address configuration.

### Phase 5: MITRE ATT&CK Tradecraft Integration
**Goal:** Research and implement full source code for specific techniques if not already covered by previous phases. Integrate these into the Spectre Implant or Team Server as appropriate.

- **TA0001 Initial Access:**
    - **T1566 Phishing:** Implement a phishing payload generator in Team Server (e.g., macro-enabled doc builder or HTML smuggler integration).
- **TA0002 Execution:**
    - **T1059 Interpreter:** Ensure `shell` module supports PowerShell execution via unmanaged PowerShell host (automating `System.Management.Automation`).
    - **T1106 Native API:** Verify direct syscall usage (already in scope, confirm completeness).
- **TA0003 Persistence:**
    - **T1547/T1053:** Implement a `persist` command module for Registry Run Keys and Scheduled Task creation.
    - **T1136:** Implement `create_user` command.
- **TA0004 Privilege Escalation:**
    - **T1055 Process Injection:** (Covered in Phase 2/3).
    - **T1548 UAC Bypass:** Implement a UAC bypass technique (e.g., Fodhelper).
- **TA0005 Defense Evasion:**
    - **T1027 Obfuscation:** (Covered by Sleep Mask/API Hashing).
    - **T1070 Indicator Removal:** Implement `timestomp` command.
    - **T1497 Sandbox Evasion:** Implement environmental keying or checks (domain join, RAM > 4GB, mouse movement).
- **TA0006 Credential Access:**
    - **T1003 Credential Dumping:** Implement `minidump` command for LSASS (using direct syscalls to avoid handle detection if possible).
- **TA0007 Discovery:**
    - **T1082/T1087/T1046:** Implement internal discovery commands (`net_scan`, `sys_info`, `user_enum`) without spawning `net.exe` (use Win32 APIs).
- **TA0008 Lateral Movement:**
    - **T1021/T1570:** Implement `psexec` style lateral movement or WMI execution.
- **TA0009 Collection:**
    - **T1113/T1056:** Implement `screenshot` and `keylogger` modules.
- **TA0010 Exfiltration:**
    - **T1048:** Ensure large file exfil is optimized over C2.
- **TA0040 Impact:**
    - **T1489:** Implement `service_stop` command.

## 4. Acceptance Criteria
- **Zero Stubs:** No function returns `Ok(())` without performing its intended action.
- **Zero TODOs:** No functional `TODO` comments remain in the codebase.
- **Zero Hardcoded Secrets:** All keys/secrets are loaded from secure sources or environment variables.
- **Full Feature Parity:** All items in the Gap Analysis are marked "FULLY IMPLEMENTED".
- **MITRE Coverage:** All listed techniques have corresponding, functional code modules.
- **Clean Build:** `cargo check` and `cargo test` pass with no warnings.
