# WRAITH-Recon Features

**Document Version:** 1.4.0 (The Complete Specification)
**Last Updated:** 2025-11-29
**Client Version:** 1.0.0

---

## 1. Advanced Network Enumeration

### 1.1 Passive Reconnaissance (Silent Mode)

**Description:** Discover assets and map network topology using purely passive techniques (zero transmission).

**Capabilities:**
*   **Promiscuous Monitoring:** Captures broadcast (ARP, DHCP, MDNS, SSDP) and multicast traffic to identify hosts without transmitting a single bit.
*   **Passive OS Fingerprinting:** Analyzes TCP SYN packets (Window Size, MSS, Options ordering, TTL) to identify OS versions (Windows, Linux, iOS, Embedded) with >90% accuracy.
*   **Passive Service Mapping:** Identifies services based on "Banner Leaks" in unencrypted traffic or TLS Certificate Handshakes (JA3/JA3S fingerprinting).
*   **Traffic Analysis:** Identifies "Top Talkers" and communication relationships (Edge Mapping).

**User Stories:**
- As an operator, I can passively map a subnet without generating ARP noise.
- As an operator, I can identify high-value targets based on traffic patterns.

### 1.2 Active Stealth Scanning

**Description:** Probe targets using advanced obfuscation to evade detection thresholds.

**Capabilities:**
*   **Timing-Obfuscated Scanning:** Scans targets using randomized inter-packet delays (Jitter) to defeat time-window detection.
*   **Decoy Scanning:** Sends spoofed packets from multiple "Decoy IPs" alongside real probes.
*   **Inverse Mapping (Firewall Walking):** Probes adjacent IPs and ports with ACK/RST packets.
*   **Stateless Probing:** Uses AF_XDP to send raw SYN/ACK/UDP probes without creating OS sockets.

**User Stories:**
- As an operator, I can perform active scanning using WRAITH's stealth transport.
- As an operator, I can verify if long-duration, low-bandwidth beacons are flagged.

**Configuration:**
```toml
[recon.scan]
mode = "stealth"  # stealth, aggressive, passive
timing_jitter = "100ms"
source_port_randomization = true
decoy_ips = ["192.168.1.50", "192.168.1.51"]
```

---

## 2. Covert Channel Simulation & Exfiltration

### 2.1 Protocol Mimicry Engine

**Description:** Test egress filtering by mimicking various legitimate protocols and traffic patterns.

**Supported Mimicry Profiles:**
*   **DNS Tunneling:** `A`, `TXT`, `CNAME` record tunneling with Base32 encoding.
*   **ICMP Tunneling:** Hiding encrypted payloads in the padding area of ICMP Echo Requests/Replies.
*   **HTTPS/TLS Mimicry:**
    *   **JA3 Mimicry:** Matches Chrome, Firefox, or Safari TLS handshake fingerprints.
    *   **Traffic Shaping:** Fits Packet Size Distribution (PSD) to legitimate profiles (e.g., YouTube stream, Azure Update).
    *   Supports Domain Fronting (sending Host header different from SNI).
*   **SMB/CIFS:** Emulation for internal lateral movement testing.

**User Stories:**
- As an operator, I can emulate DNS-over-HTTPS traffic to bypass firewall rules.
- As an operator, I can test if ICMP tunneling is detected by the IDS.

### 2.2 Data Exfiltration Assessment (DLP Testing)

**Description:** Simulate unauthorized data transfer to test Data Loss Prevention (DLP) systems.

**Exfiltration Modes:**
*   **Burst:** High-speed transfer (testing throughput limits).
*   **Drip (Slow-Drip):** Ultra-low-bandwidth transfer (e.g., 1 byte per hour) to bypass "Top Talker" reports.
*   **Fragmented:** Out-of-order packet transmission to test reassembly.
*   **Steganographic:** Hiding data in cover traffic headers (e.g., HTTP headers, DNS padding).
*   **Multi-Path:** Splits a single file transfer across multiple protocols and paths simultaneously (e.g., 50% DNS, 50% HTTPS).

**Safety Control:** All "sensitive" data used is synthetically generated dummy data (PII, PCI).

**User Stories:**
- As an operator, I can attempt to transfer dummy PII data to verify DLP blocking.
- As an operator, I can fragment a large file into 1-byte chunks to test reassembly detection.

---

## 3. Egress Path Analysis

**Description:** Map and validate potential paths for data to leave the protected network.

**Analysis Capabilities:**
*   **Egress Scanner:** Checks all 65535 ports for outbound connectivity.
*   **Proxy Detection:** Identifies upstream proxies and authentication requirements.
*   **SSL Inspection Check:** Verifies certificate chains for MITM appliances.
*   **NAT Type Discovery:** Classifies NAT behavior (Full Cone, Restricted, Symmetric).

**User Stories:**
- As an operator, I can identify which outbound ports are allowed through the firewall.
- As an operator, I can detect transparent interception (SSL inspection).

---

## 4. Defense Stress Testing

**Description:** Stress test network security appliances (firewalls, IPS/IDS) with high-throughput WRAITH traffic.

**Modes:**
*   **Throughput Test:** Saturation testing using `wraith-transport` (AF_XDP) to generate 10Gbps+ traffic.
*   **State Exhaustion:** Generate millions of unique source IP/Port combinations (spoofed) to fill connection tracking tables.
*   **Jitter Flood:** Rapidly changing packet timing to confuse heuristic analyzers.

**User Stories:**
- As an operator, I can generate 10Gbps of encrypted traffic to test firewall throughput.
- As an operator, I can flood state tables with unique connection IDs.

---

## 5. Governance & Safety (RoE Enforcement)

### 5.1 Engagement Scoping

**Description:** Strict software controls to limit operations to authorized targets.

**Features:**
*   **Target Whitelisting:** CIDR blocks and domains allowed for interaction.
*   **Blacklisting:** Explicitly excluded critical infrastructure IPs.
*   **Time-Fencing:** Operations automatically cease outside defined windows (e.g., "M-F, 09:00-17:00").
*   **Kill Switch:** Cryptographically signed command (via UDP Broadcast or DNS TXT) to immediately halt all activity.

**Config Example:**
```toml
[scope]
allowed_cidrs = ["10.10.0.0/16"]
excluded_ips = ["10.10.1.5"]
expiry = "2025-12-31T23:59:59Z"

[governance]
kill_switch_method = "dns_txt"
kill_switch_domain = "_kill.ops.wraith.io"
```

### 5.2 Audit Logging

**Description:** Comprehensive, tamper-evident logging of all actions.

**Features:**
*   **Command Log:** Every CLI command executed.
*   **Traffic Log:** Metadata of all generated network traffic (PCAP compatible).
*   **Hash Chaining:** Logs are cryptographically linked to prevent modification.

---

## 6. User Interface

### CLI Dashboard

WRAITH-Recon runs primarily as a CLI tool with an ncurses-based dashboard for real-time monitoring.

```text
+-----------------------------------------------------------------------------+
| WRAITH-RECON v1.0.0  |  Mode: STEALTH  |  Status: ACTIVE (Scope Valid)      |
+-----------------------------------------------------------------------------+
| Target: 10.10.50.0/24 (HR Dept)                                             |
|                                                                             |
| [DISCOVERY]                                                                 |
| > Passive: 15 hosts identified                                              |
| > Active:  3 web servers (80, 443)                                          |
|                                                                             |
| [EXFILTRATION TEST]                                                         |
| > Job ID: XF-992                                                            |
| > Method: DNS Tunneling (Mimicry)                                           |
| > Payload: Synthetic_PII.zip (50MB)                                         |
| > Progress: [=================>        ] 65%                                |
| > Rate: 15 KB/s (Throttled)                                                 |
|                                                                             |
| [ALERTS]                                                                    |
| [!] Connection reset on 10.10.50.5:443 (Possible IPS block)                 |
+-----------------------------------------------------------------------------+
| > set mode aggressive                                                       |
+-----------------------------------------------------------------------------+
```

---

## See Also
- [Architecture](architecture.md)
- [Implementation](implementation.md)
- [Client Overview](../overview.md)
