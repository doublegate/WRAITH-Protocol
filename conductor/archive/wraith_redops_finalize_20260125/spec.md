# Specification: Finalize WRAITH-RedOps Implementation

## Overview
This track aims to eliminate all remaining stub and skeleton implementations within the WRAITH-RedOps platform. Following the previous remediation efforts, this phase focuses on implementing high-complexity features (BOF Loader, Indirect Syscall Injection, SMB Pivoting) to ensure the platform is fully operational and adheres to Tier 3 offensive security standards.

## Functional Requirements

### 1. Spectre Implant Finalization
- **BOF Loader (COFF):**
    - Implement a fully functional COFF loader in `no_std` Rust.
    - Support section mapping (Code, Data, Read-Only).
    - Implement relocation handling (IMAGE_REL_AMD64_ADDR64, IMAGE_REL_AMD64_REL32, etc.).
    - Implement symbol resolution for Beacon APIs (BeaconPrintf, BeaconOutput, etc.).
- **Stealth Process Injection:**
    - Implement Reflective DLL Injection using Indirect Syscalls (NtCreateSection, NtMapViewOfSection).
    - Implement Process Hollowing using Indirect Syscalls (NtCreateUserProcess, NtUnmapViewOfSection).
    - Implement Thread Hijacking (NtGetContextThread, NtSetContextThread).
    - Ensure all Windows API calls for injection are routed through the Hell's Gate/Halo's Gate syscall engine.
- **SOCKS Proxy:**
    - Implement a full SOCKS5 state machine (Greeting -> Auth -> Connect -> Forwarding).
    - Integrate with the C2 channel to tunnel TCP traffic from the Team Server.
- **Interactive Shell:**
    - Implement PTY/Pipe-based shell execution for both Linux and Windows.

### 2. Team Server Finalization
- **SMB Pivoting Logic:**
    - Implement the "Pivot Only" architecture. The Team Server will manage SMB tasks by routing them through an established "parent" session to a "child" implant over a named pipe.
    - Update `listeners/smb.rs` to handle session-linked pipe communication instead of a standalone server.
- **DNS Tunneling Completion:**
    - Implement full TXT record parsing and assembly for large WRAITH frames over DNS.
- **Advanced Builder Pipeline:**
    - Transition from simple byte-patching to a full LLVM-based build pipeline.
    - Support compile-time obfuscation passes (string encryption, junk code injection).

### 3. Operator Client Finalization
- **Operational Dashboard:**
    - Implement real-time metrics (Beacon count, Task throughput, Log frequency).
- **Configuration UI:**
    - Replace hardcoded connection strings with a dynamic settings manager.

## Non-Functional Requirements
- **no_std Compliance:** All implant modules must remain strictly `no_std`.
- **Evasion Parity:** Evasion techniques (Indirect Syscalls, Sleep Masking) must be consistently applied across all injection modules.

## Acceptance Criteria
- **Zero Stubs:** No `Ok(())` or `vec![]` placeholders remain in critical functional paths.
- **BOF Execution:** Successfully execute a Cobalt Strike compatible BOF (e.g., `whoami`) and retrieve output.
- **Stealth Injection:** Successfully inject into a remote process without calling hooked Ntdll APIs directly.
- **P2P SMB:** Establish a child beacon via an SMB named pipe routed through a parent UDP beacon.
