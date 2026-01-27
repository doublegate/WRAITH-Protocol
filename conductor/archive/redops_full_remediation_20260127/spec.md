# Specification: WRAITH-RedOps Full Remediation & Aspirational Integration

## Overview
Complete the WRAITH-RedOps implementation by methodically remediating all gaps identified in `GAP-ANALYSIS-v2.3.0.md` and `GAP-ANALYSIS-v2.2.5.md`. This track includes the delivery of high-priority features, platform completeness fixes, advanced tradecraft, and the integration of high-performance architectural aspirations (AF_XDP, io_uring, etc.) to bring RedOps into full alignment with the primary WRAITH Protocol vision.

## Functional Requirements

### Phase 1: Core Functionality & Protocol Acceleration
- **P1 Remediation:** Implement full Noise DH key ratcheting (Rekey every 2 minutes or 1M packets).
- **Aspirational Integration:** Integrate **AF_XDP Kernel Bypass** for zero-copy packet I/O in RedOps transport.
- **Aspirational Integration:** Implement **io_uring** for asynchronous network and file I/O operations (Linux 6.2+).
- **Aspirational Integration:** Integrate **BBR Congestion Control** for optimized C2 throughput.

### Phase 2: PowerShell Execution & High-Priority Fixes
- **P1 Remediation:** Replace the `RUNNER_DLL` placeholder with a fully compiled .NET PowerShell runner assembly.
- **P2 Fixes:** Correct CLR GUID for runtime host, fix SMB listener `.unwrap()` panics, and resolve the HMAC fallback security risk in the start script.
- **Phishing Builder:** Complete the VBA shellcode runner stub in `builder/phishing.rs`.

### Phase 3: Platform Completeness & Evasion
- **Evasion Enhancements:** Implement dynamic heap and .text section discovery for the Sleep Mask (replacing hardcoded bases).
- **Build Pipeline:** Implement LLVM-level obfuscation passes via RUSTFLAGS in the builder.
- **ARM64 Support:** Implement hardware-based entropy using `CNTVCT_EL0` for ARM64 targets.
- **Aspirational Integration:** Support **Multi-Transport Failover** (autonomous switching between HTTP/DNS/UDP/SMB).

### Phase 4: Advanced Features & Distributed Discovery
- **Mesh C2:** Implement P2P Mesh routing, orchestration, and automated topology building.
- **Aspirational Integration:** Integrate **Kademlia DHT** for decentralized peer discovery within the RedOps network.
- **Persistence & Discovery:** Implement a persistent keylogger with configurable intervals and PEB-based ImageBase queries.
- **Operator Experience:** Add a Settings/Preferences UI to the Tauri client for persistent configuration.
- **Test Coverage backfill:** Remediate coverage for existing code to meet the >80% project standard.

## Non-Functional Requirements
- **Zero Stubs Policy:** In accordance with the `workflow.md`, no placeholders, skeleton implementations, or "coming soon" stubs are permitted.
- **Wire-Speed Performance:** Maintain the 10+ Gbps throughput capability through proper AF_XDP implementation.
- **Stealth:** Ensure all evasion techniques (Sleep Mask, Hashing, Syscalls) are production-grade and bypass standard hooks.

## Acceptance Criteria
- [ ] All findings in `GAP-ANALYSIS-v2.3.0.md` and `GAP-ANALYSIS-v2.2.5.md` are documented as RESOLVED in the source code.
- [ ] All "Aspirational" features (AF_XDP, io_uring, BBR, DHT, Failover) are fully functional and integrated.
- [ ] The `start_redops.sh` script successfully deploys the full infrastructure.
- [ ] Test coverage for `wraith-redops` components exceeds 80%.
- [ ] Zero clippy warnings across all RedOps-related crates.

## Out of Scope
- Implementation of MITRE ATT&CK techniques not explicitly mentioned in the gap analysis or primary design documents.
