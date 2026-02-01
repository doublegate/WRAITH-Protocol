# Track Specification: WRAITH-RedOps Gap Remediation & Polish

**Track ID:** `redops_gap_remediation_20260201`
**Type:** Enhancement / Refactor
**Created:** 2026-02-01

## Overview
This track addresses the final 10 findings identified in `GAP-ANALYSIS-v2.3.6.md` to bring the WRAITH-RedOps platform to 100% completion and full production readiness. It focuses on hardening the Mesh network cryptography, expanding test coverage for new modules, and polishing the Operator Client and Implant for enhanced stealth and usability. Additionally, it includes a requested dependency audit and specific integration tests for the new Mesh security features.

## Goals
1.  **Harden Mesh Security:** Replace static XOR keys with per-campaign AEAD encryption derived via KDF.
2.  **Expand Test Coverage:** Implement robust testing for `Token`, `Transform`, `SideLoad`, and `Ingress` modules, ensuring CI stability on Linux while maximizing fidelity on Windows.
3.  **Enhance IPv6 Support:** Add IPv6 capabilities to C2 parsing and SOCKS proxying.
4.  **Polish Tradecraft:** Upgrade stack spoofing to ROP-based implementation and explicit memory zeroization.
5.  **Clean Codebase:** Resolve comment ambiguities and frontend logging noise.
6.  **Verify & Audit:** Perform a dependency audit and comprehensive integration testing.

## Functional Requirements

### 1. Mesh Cryptography Upgrade (P3-NEW-1)
-   **Derivation:** Implement `derive_mesh_key(campaign_id, salt)` using BLAKE3 or Argon2 at build time.
-   **Encryption:** Replace static XOR with XChaCha20-Poly1305 (or compatible lightweight AEAD) for Mesh frames.
-   **Verification:** Ensure the implant rejects packets that fail authentication tag verification.

### 2. Module Testing (P3-NEW-2)
-   **Strategy:** Implement a hybrid testing approach:
    -   **Logic Tests:** Platform-agnostic tests for parsing, decoding, and state management (run everywhere).
    -   **OS-Gated Tests:** `#[cfg(target_os = "windows")]` tests for real API interactions (skipped on Linux CI, run on Windows).
-   **Coverage:** `Token` (permissions), `Transform` (decoding correctness), `SideLoad` (path logic), `Ingress` (url parsing).

### 3. Networking Enhancements (P4-SI-1, P4-SI-4)
-   **C2 Parsing:** Update `c2/mod.rs` to correctly parse and handle IPv6 addresses (e.g., `[::1]:8080`).
-   **SOCKS Proxy:** Update `socks.rs` to support SOCKS5 IPv6 address type (`0x04`) and connectivity.

### 4. Tradecraft & Polish (P4-SI-2, P4-SI-3, P4-SI-5)
-   **Killswitch:** Add timezone awareness to date parsing in `c2/mod.rs`.
-   **Stack Spoofing:** Refactor `obfuscation.rs` to use a ROP-based chain for the sleep mask, improving evasion analysis resistance.
-   **Zeroization:** Explicitly call `zeroize()` on the decoded buffer in `transform.rs` before it is dropped.

### 5. Documentation & Frontend (P4-SI-7, P4-SI-8, P4-OC-1)
-   **Comments:** Clarify "simulates" comments in `exfiltration.rs` and `impact.rs` to accurately reflect functionality (e.g., "Implements DNS exfiltration pattern").
-   **Console:** Remove or wrap `console.error` calls in `DiscoveryDashboard.tsx` and `LootGallery.tsx` behind a debug flag or proper logger.

## Non-Functional Requirements
-   **CI Stability:** Changes must not break the Linux-based Github Actions CI. Windows-specific tests must be properly gated.
-   **Performance:** ROP stack spoofing must not introduce significant latency to the sleep cycle.
-   **Compatibility:** IPv6 changes must maintain backward compatibility with IPv4.

## Out of Scope
-   New major feature development (new modules beyond those existing).
-   UI design overhauls.

## Acceptance Criteria
-   [ ] **Mesh Key:** Implant successfully derives key from Campaign ID and encrypts/decrypts mesh packets. Static "WRAITH_MESH_KEY_2026" string is removed.
-   [ ] **Tests:** New tests for `Token`, `Transform`, `SideLoad`, `Ingress` exist and pass. Linux CI remains green.
-   [ ] **IPv6:** Implant can parse IPv6 C2 addresses and proxy IPv6 traffic via SOCKS.
-   [ ] **Security:** Stack spoofing uses ROP chain. `decode_base64` zeroizes memory.
-   [ ] **Cleanliness:** No `console.error` in production build. Comments are accurate.
-   [ ] **Audit:** Dependency audit report generated and high-severity issues (if any) addressed.
