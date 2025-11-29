# WRAITH-Recon Integration Guide

**Document Version:** 1.0.0
**Last Updated:** 2025-11-29

---

## 1. Integration Overview

WRAITH-Recon is designed to fit into a modern offensive security ecosystem. It can consume intelligence from other tools (e.g., scope definitions) and output structured data for analysis platforms (SIEM, Reporting tools).

---

## 2. Input Integrations

### 2.1 Scope Import
WRAITH-Recon can import scope definitions from standard formats.

*   **Nmap XML:**
    ```bash
    wraith-recon import --format nmap --input scan_results.xml --sign key.pem
    ```
    *Parses `<address addr="..." />` tags to generate an allowed CIDR list.*

*   **CSV (Asset Inventory):**
    *   Format: `IP,Hostname,Owner,Criticality`
    *   Usage: Tags assets in the internal DB with "Criticality" levels to adjust scan aggression.

### 2.2 Configuration Management
*   **Ansible/Terraform:**
    *   The `config.signed` file can be deployed via standard CM tools.
    *   The binary supports reading config from `STDIN` for pipeline integration.

---

## 3. Output Integrations

### 3.1 SIEM / Log Aggregation
WRAITH-Recon supports structured JSON logging for ingestion into ELK, Splunk, or Graylog.

**Log Format (JSON Line):**
```json
{
  "timestamp": "2025-11-29T10:00:00Z",
  "level": "INFO",
  "module": "active_scanner",
  "event": "port_open",
  "data": {
    "target": "10.0.0.5",
    "port": 443,
    "proto": "tcp",
    "fingerprint": "nginx/1.18"
  },
  "trace_id": "scan-001"
}
```

**Splunk Integration:**
*   Configure `fluend` or `filebeat` to tail `/var/log/wraith/findings.json`.
*   Dashboards can visualize "Open Ports by Subnet" or "Detected OS Distribution".

### 3.2 Vulnerability Scanners
WRAITH-Recon is *not* a vuln scanner, but it feeds them.

*   **Output:** `hosts.txt` (List of live IPs).
*   **Integration:**
    ```bash
    wraith-recon --mode active --out live_hosts.txt
    nessus-cli --input live_hosts.txt --launch "Basic Scan"
    ```

---

## 4. Interoperability with WRAITH Ecosystem

### 4.1 WRAITH-RedOps
*   **Role:** WRAITH-Recon acts as the "Eyes" for RedOps "Hands".
*   **Workflow:**
    1.  Recon maps the network and identifies a Gateway.
    2.  Recon exports the Gateway IP and open UDP port.
    3.  RedOps configures a listener profile matching that open port.

### 4.2 WRAITH-Relay
*   WRAITH-Recon can route its scan traffic *through* a WRAITH-Relay mesh to anonymize the source IP.
*   **Config:** `[transport] proxy = "wraith://10.10.10.10:9000"`

---

## 5. API Reference (IPC)

WRAITH-Recon exposes a local Unix Domain Socket for IPC control (if enabled).

**Endpoint:** `/var/run/wraith-recon.sock`
**Protocol:** JSON-RPC 2.0

**Methods:**
*   `status()`: Returns current scan progress.
*   `pause()`: Temporarily halts packet generation.
*   `resume()`: Resumes operations.
*   `dump_assets()`: Returns the current Asset Graph JSON.
