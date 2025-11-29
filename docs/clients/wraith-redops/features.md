# WRAITH-RedOps Features

**Document Version:** 1.4.0 (The Complete Specification)
**Last Updated:** 2025-11-29
**Client Version:** 1.0.0

---

## 1. WRAITH-Native C2 Channels

**Description:** Use the WRAITH protocol as the primary or fallback command channel for resilient, unblockable communications.

**Channel Types:**
*   **Direct UDP:** Standard WRAITH encrypted transport (Noise_XX handshake, Chacha20-Poly1305).
*   **Covert:** DNS/ICMP/HTTPS mimicry (via `wraith-obfuscation`).
*   **P2P:** SMB (Named Pipes) or WRAITH-over-TCP for internal peer-to-peer chaining (lateral movement).

**User Stories:**
- As an operator, I can maintain control of an implant even if TCP/443 is blocked, using WRAITH's UDP/covert modes.
- As an operator, I can route C2 traffic through a mesh of WRAITH relays to obscure the Team Server IP.

---

## 2. The "Spectre" Implant

**Description:** A modular, memory-resident agent written in `no_std` Rust for maximum stealth and stability.

### 2.1 Execution & Injection
*   **Beacon Object Files (BOF):** Loads and executes COFF object files in memory without linking. Compatible with industry-standard BOF collections (Cobalt Strike compatible API).
*   **Reflective Loading:** Can load DLLs from memory without touching disk.
*   **.NET Hosting:** Hosting CLR to run C# assemblies in memory (Windows Only).
*   **Process Injection:** Reflective DLL injection, hollow process injection.

**User Stories:**
- As an operator, I can inject the Spectre agent into a running process.
- As an operator, I can load additional capabilities (modules) at runtime without touching the disk.

### 2.2 Evasion & Tradecraft (Advanced)
*   **Sleep Mask (Obfuscation):** Encrypts heap and executable sections (RX -> RW -> Encrypt -> Sleep -> Decrypt -> RX) during sleep cycles to evade memory scanners.
*   **Stack Spoofing:** Rewrites call stack frames to look like legitimate system calls (e.g., `WaitForSingleObject`).
*   **Indirect Syscalls:** Bypasses EDR user-land hooks (ntdll.dll) by invoking syscalls directly (Hell's Gate technique).
*   **AMSI/ETW Patching:** Temporarily disables Windows logging and scanning interfaces in the local process memory.
*   **Kerberos Manipulation:** (New) Support for Pass-the-Ticket (PTT) and Overpass-the-Hash (OTH) attacks via BOF integration.
*   **Token Manipulation:** Steal and impersonate tokens from other processes (`SeDebugPrivilege` required).

### 2.3 File System & Network
*   **VFS Abstraction:** Upload/Download/List files.
*   **Shell Integration:** PTY-supported interactive shell access.
*   **SOCKS Proxy:** Tunnel operator traffic through the beacon (SOCKS4a/5).

---

## 3. Collaborative Team Server

**Description:** A multi-user server that manages listeners, implants, and data.

**Capabilities:**
*   **Real-time Sync:** All operators see the same data instantly via WebSocket.
*   **Role-Based Access:** Admin, Operator, Read-Only roles.
*   **Deconfliction Server:** Built-in mechanism to register targets and prevent collisions.
*   **Data Aggregation:** Centralized logging of all keystrokes, screenshots, credentials, and downloads.

**User Stories:**
- As a Red Team Lead, I can see all active sessions from my team members.
- As an operator, I can chat with other operators within the client.

---

## 4. Automation & Scripting

**Description:** Automate common tasks and adversary TTPs.

**Capabilities:**
*   **Scripting Bridge:** Aggressor-style Lua or Python bridge for client automation (hook into events like "On Beacon Initial Checkin").
*   **Task Queuing:** Queue commands for asynchronous execution when the agent checks in.
*   **Playbooks:** Pre-defined sequences of TTPs (e.g., "APT29 Enumeration Sequence").
*   **"Ghost" Replay:** Replay a sequence of TTPs exactly as they occurred in a previous engagement for verification or training.

**User Stories:**
- As an operator, I can write a script to automatically survey a host upon check-in.
- As an operator, I can define "Auto-Kill" rules if an implant detects it's in a sandbox.

---

## 5. Governance & Safety

**Description:** Strict controls to ensure the red team operates within legal and ethical boundaries.

**Features:**
*   **Hardcoded Scope:** Implants refuse to execute against IPs/Domains not in the compiled configuration (Kernel-side block).
*   **Time-to-Live (TTL):** Implants self-destruct after a specific date.
*   **Execution Guard:** Prevents execution of high-risk commands (e.g., `rm -rf /`) without 2-person authorization.
*   **Audit Trail:** Immutable, append-only logs of every command sent and byte received.

---

## 6. User Interface

### Operator Client

The client is a cross-platform GUI (Tauri + React) designed for density of information and speed.

```
+--------------------------------------------------------------------------------+
| WRAITH-REDOPS | Campaign: OPERATION_SKYFALL | Users: 4 | Listeners: 2 active   |
+--------------------------------------------------------------------------------+
| [Sessions] [Graph] [Targets] [Listeners] [Reporting]                           |
+--------------------------------------------------------------------------------+
| ID   | User      | PID  | Arch  | IP Address    | Last | Status                |
|------+-----------+------+-------+---------------+------+-----------------------|
| 0x01 | SYSTEM    | 442  | x64   | 10.10.50.5    | 2ms  | [Admin] Interactive   |
| 0x02 | jdoe      | 2210 | x64   | 10.10.50.12   | 5s   | [User]  Sleep(5s)     |
| 0x03 | web_svc   | 991  | x86   | 192.168.1.5   | 1m   | [Svc]   Unlinked      |
+--------------------------------------------------------------------------------+
| [Session 0x01 - SYSTEM@10.10.50.5]                                             |
| > upload /opt/tools/mimikatz.exe C:\Windows\Temp\m.exe                         |
| [*] Upload started...                                                          |
| [+] Upload complete (2.1MB)                                                    |
| > execute C:\Windows\Temp\m.exe sekurlsa::logonpasswords                       |
| [*] Tasked beacon to execute...                                                |
| [+] Output received:                                                           |
|     Authentication Id : 0;1337                                                 |
|     Package Name      : NTLM                                                   |
|     User Name         : Administrator                                          |
|                                                                                |
| [input command...]                                                             |
+--------------------------------------------------------------------------------+
```

---

## See Also
- [Architecture](architecture.md)
- [Implementation](implementation.md)
- [Client Overview](../overview.md)
