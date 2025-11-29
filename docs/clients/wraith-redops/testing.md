# WRAITH-RedOps Testing Strategy

**Document Version:** 1.0.0
**Last Updated:** 2025-11-29
**Governance:** See [Security Testing Parameters](../../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)

---

## 1. Overview
This document outlines the validation strategy for the WRAITH-RedOps platform. Testing focuses on **operational stability**, **stealth/evasion effectiveness**, and **governance safety**.

---

## 2. Unit Testing (Rust)

### 2.1 Implant Logic (`no_std`)
*   **Goal:** Ensure core logic works without OS dependencies.
*   **Tool:** `cargo test` with custom runner for freestanding targets.
*   **Scope:**
    *   Protocol serialization/deserialization (PDU).
    *   Crypto primitives (ChaCha20, Noise Handshake).
    *   Command dispatching logic.

### 2.2 Team Server Logic
*   **Goal:** Verify state management and concurrency.
*   **Scope:**
    *   Database transactions (Task queuing, Result processing).
    *   Listener Bus routing (ensure packets go to correct session).
    *   Builder pipeline (verify artifact generation outputs valid PE files).

---

## 3. Integration Testing (Lab Environment)

### 3.1 End-to-End Connectivity
*   **Setup:** 
    *   Team Server (Ubuntu).
    *   Target VM (Windows 10).
*   **Procedure:**
    1.  Generate payload.
    2.  Execute on Target.
    3.  Verify "New Beacon" event on Server.
    4.  Task `whoami`.
    5.  Verify output received.

### 3.2 Governance Verification
*   **Goal:** Ensure "Scope Lock" works.
*   **Procedure:**
    1.  Compile implant with Allowed CIDR `10.0.0.0/24`.
    2.  Deploy on `192.168.1.50` (Out of Scope).
    3.  Assert: Implant refuses to run or terminates immediately.
    4.  Task implant to `portscan 8.8.8.8` (Out of Scope).
    5.  Assert: Task rejected by Implant Kernel.

---

## 4. Adversary Simulation (Purple Team)

### 4.1 Evasion Testing (vs EDR)
*   **Goal:** Verify Sleep Mask and Syscalls bypass detection.
*   **Environment:** Lab with Defender for Endpoint, CrowdStrike, or SentinelOne (Trial).
*   **Metrics:**
    *   **Static Detection:** Does the file get eaten on disk? (Test Obfuscator).
    *   **Dynamic Detection:** Does `shell whoami` trigger an alert?
    *   **Memory Scanning:** run `pe-sieve` against the beacon process while it is sleeping. Assert: No malicious patterns found.

### 4.2 Network Stealth
*   **Goal:** Verify C2 traffic blends in.
*   **Tool:** RITA (Real Intelligence Threat Analytics) / Zeek.
*   **Procedure:**
    1.  Run beacon for 24 hours with `jitter = 20%`.
    2.  Analyze PCAPs with RITA.
    3.  Assert: Beacon score is low (< 0.5).

---

## 5. CI/CD Pipeline

*   **Build:** Cross-compile Implant (Windows/Linux) and Server.
*   **Test:** Run Unit Tests.
*   **Safety Check:** Verify Governance module cannot be disabled via simple flag.
*   **Release:** Sign binaries with dev key.
