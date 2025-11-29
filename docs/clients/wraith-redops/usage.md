# WRAITH-RedOps Operations Guide

**Document Version:** 1.0.0
**Last Updated:** 2025-11-29

---

## 1. Infrastructure Deployment

### 1.1 Team Server Setup
*   **OS:** Debian 12 / Ubuntu 22.04 (Hardened).
*   **Hardware:** 4 vCPU, 8GB RAM (Minimum).
*   **Command:**
    ```bash
    # 1. Install Dependencies
    apt install postgresql mingw-w64 clang llvm lld

    # 2. Initialize Database
    sudo -u postgres psql -f setup_schema.sql

    # 3. Start Server
    ./wraith-teamserver --config server.toml
    ```

### 1.2 Redirector Setup
Never expose the Team Server directly. Use Redirectors.
*   **Method A (Dumb Pipe):** `socat` forwarding UDP/443 to Team Server.
*   **Method B (Smart Filter):** `nginx` reverse proxy (for HTTPS) or `iptables` rules that only allow traffic matching specific packet sizes/headers.

---

## 2. Campaign Workflow

### 2.1 Listener Configuration
1.  Open Client -> Listeners -> Add.
2.  Type: `WRAITH_UDP`.
3.  Port: `443` (Bind to 0.0.0.0).
4.  Host: `c2.example.com` (The public DNS of your redirector).

### 2.2 Payload Generation
1.  Open Client -> Attacks -> Packages -> Windows EXE (S).
2.  Listener: Select the listener created above.
3.  Arch: `x64`.
4.  Output: `payload.exe`.

### 2.3 Execution & Management
1.  Execute `payload.exe` on target VM.
2.  Wait for "New Beacon" notification in Client.
3.  Right-click Beacon -> Interact.
4.  **Common Commands:**
    *   `sleep 10 20` (Sleep 10s with 20% jitter).
    *   `ls` (List files).
    *   `upload /local/path /remote/path`.
    *   `job-shell whoami` (Run command in separate thread).

---

## 3. Tradecraft Guidelines (OpSec)

### 3.1 Memory Scanning
*   **Risk:** EDRs scan memory for unbacked executable code (RWX pages).
*   **Mitigation:** Always enable `sleep_mask = true` in profile. This encrypts the beacon when not executing commands.

### 3.2 Network Evasion
*   **Risk:** Beaconing periodicity (heartbeats) detected by SIEM.
*   **Mitigation:** Use high jitter (>20%) and long sleep intervals (>60s) for long-haul persistence. Use `interactive` mode only briefly.

### 3.3 Binary Signatures
*   **Risk:** Antivirus static signatures.
*   **Mitigation:** Never reuse the same binary. The Builder generates unique hashes. Use "Artifact Kit" to modify the loader stub logic if signatures are detected.

---

## 4. Troubleshooting

### 4.1 "Beacon Not Checking In"
1.  Check **Firewall** on Target (Is UDP/443 outbound allowed?).
2.  Check **DNS** resolution of C2 domain on Target.
3.  Check **Team Server Logs** (`logs/server.log`) for handshake errors (bad key?).

### 4.2 "Injection Failed"
*   Cause: Target process architecture mismatch (x86 vs x64) or Protected Process Light (PPL) protections.
*   Fix: Use `ps` to find a suitable process (e.g., `explorer.exe` user-level) of the same arch.
