# Track Specification: APT Playbooks & SMB2 Hardening

## 1. Overview
This track completes the advanced tradecraft and automation features of WRAITH-RedOps by implementing the APT Playbook system and transitioning the SMB C2 channel to a full, hardened SMB2 state machine implementation.

## 2. Functional Requirements

### 2.1 APT Playbook Sequence Implementation
*   **Playbook Templates:** Implement support for loading read-only attack sequence templates from a `playbooks/` directory (supporting both YAML `.yaml` and JSON `.json` formats) on Team Server startup.
*   **Instantiation:** Allow operators to instantiate a Playbook into an editable "Attack Chain".
*   **Predefined Library:** Include a set of standard APT emulation playbooks (e.g., "APT29 Discovery & Collect", "Generic Ransomware Simulation").
*   **Database Schema:** Update schema to distinguish between `Playbook` (template) and `AttackChain` (instance).

### 2.2 SMB2 Protocol Hardening (Full State Machine)
*   **Mimicry:** Replace current simplified framing with a full SMB2 header (`0xFE 'S' 'M' 'B'`) and state machine.
*   **State Machine:** Implement the following SMB2 transitions:
    *   `SMB2 NEGOTIATE`
    *   `SMB2 SESSION_SETUP` (Simplified NTLMSSP or anonymous)
    *   `SMB2 TREE_CONNECT` (Connect to a named pipe share)
    *   `SMB2 CREATE` / `SMB2 WRITE` / `SMB2 READ` (C2 traffic carried inside pipe operations)
*   **Spectre Implementation:** Implement these structures manually in `no_std` Rust using raw syscalls for socket I/O.
*   **Team Server Implementation:** Update `smb.rs` listener to handle the full handshake sequence.

## 3. Technical Constraints
*   **Zero Stub Policy:** Every packet structure and state transition must be real. No "TODO" for auth or negotiation.
*   **Stealth:** The SMB traffic must pass basic protocol validation by tools like Wireshark or Zeek.

## 4. Acceptance Criteria
*   **Playbooks:** Operators can list and select templates in the Console and create chains from them.
*   **SMB2 Handshake:** Wireshark identifies the C2 session as a legitimate SMB2 stream.
*   **no_std Compatibility:** Spectre implant builds and executes the full SMB2 sequence on Windows and Linux targets.
*   **Robustness:** Handshake handles network timeouts and malformed responses gracefully.
