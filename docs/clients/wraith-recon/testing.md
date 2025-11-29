# WRAITH-Recon Testing Strategy

**Document Version:** 1.0.0
**Last Updated:** 2025-11-29
**Governance:** See [Security Testing Parameters](../../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)

---

## 1. Overview
This document outlines the validation strategy for WRAITH-Recon. Given the tool's capability to generate high-velocity network traffic and its use in sensitive engagements, testing must rigorously verify **safety**, **stealth**, and **correctness**.

---

## 2. Safety Verification (The "Kill Switch" Tests)
These tests ensure the tool **never** violates the Rules of Engagement (RoE).

### 2.1 Governance Unit Tests
*   **Goal:** Verify the `SafetyController` logic.
*   **Method:** Property-based testing (using `proptest`).
*   **Scenarios:**
    *   **CIDR Boundary:** Generate random IPs; assert `check()` only returns true if IP is in allowlist.
    *   **Blacklist Priority:** Generate IP present in *both* allowlist and blacklist; assert `check()` returns false.
    *   **Expiry:** Mock system clock to `expiry + 1s`; assert `check()` returns false.
    *   **Kill Switch:** Set `KILL_SWITCH` atomic; assert `check()` returns false for *any* IP.

### 2.2 Fuzzing the Configuration Parser
*   **Goal:** Ensure malformed or malicious config files cannot crash the agent or bypass checks.
*   **Tool:** `cargo-fuzz`.
*   **Targets:** `Config::parse()`, `Signature::verify()`.

---

## 3. Network Capability Testing

### 3.1 AF_XDP Loopback Test
*   **Goal:** Verify kernel-bypass read/write without physical network hardware.
*   **Setup:** Use `veth` pairs in a network namespace.
*   **Procedure:**
    1.  Create namespace `ns1`.
    2.  Bind WRAITH-Recon to `veth0`.
    3.  Run `tcpdump` on `veth1`.
    4.  Send 1M packets.
    5.  Verify packet count and data integrity.

### 3.2 Throughput Benchmark
*   **Goal:** Validate 10Gbps capability.
*   **Environment:** Bare-metal Linux server with Intel X520/X710 NIC.
*   **Metric:** Packet Per Second (PPS) vs CPU usage.
*   **Pass Criteria:** > 10M PPS at < 50% CPU on 1 core.

---

## 4. Detection & Stealth Testing

### 4.1 Blue Team Simulation
*   **Goal:** Verify effectiveness of Obfuscation profiles.
*   **Setup:**
    *   **Attacker:** WRAITH-Recon running "Stealth Scan".
    *   **Defender:** Snort/Suricata with "ET Open" and "Snort Subscriber" rulesets.
*   **Scenarios:**
    1.  **Baseline:** Run `nmap -sS -T4`. Expect: Detection.
    2.  **Test:** Run `wraith-recon --profile stealth`. Expect: No Alerts.
    3.  **Test:** Run `wraith-recon --profile jitter-pareto`. Expect: No Behavioral Alerts.

### 4.2 Mimicry Validation
*   **Goal:** Ensure "DNS" traffic looks like DNS.
*   **Tool:** Wireshark / `tshark`.
*   **Procedure:**
    1.  Capture generated traffic.
    2.  Run `tshark -r capture.pcap -V`.
    3.  Assert: No "Malformed Packet" errors.
    4.  Assert: All fields (Flags, Opcode, RCODE) match RFC 1035.

---

## 5. Integration Testing

### 5.1 End-to-End Exfiltration
*   **Setup:**
    *   Client: WRAITH-Recon (Exfil Mode).
    *   Server: WRAITH-Listener (with reassembly logic).
*   **Procedure:**
    1.  Generate 100MB random file.
    2.  Exfiltrate via DNS Tunnel.
    3.  Compare SHA256 of source and destination files.
    *   **Pass Criteria:** Hashes match.

---

## 6. CI/CD Pipeline

*   **Stage 1: Static Analysis** (`cargo clippy`, `cargo fmt`, `audit`).
*   **Stage 2: Unit Tests** (`cargo test`).
*   **Stage 3: Safety Tests** (Governance logic verification).
*   **Stage 4: Build** (Release binary generation).
*   **Stage 5: Artifact Signing** (Sign binary with dev key).
