# WRAITH-RedOps Reference Architecture

**Document Version:** 1.3.0 (Technical Deep Dive)
**Last Updated:** 2025-11-29
**Classification:** Reference Architecture
**Governance:** See [Security Testing Parameters](../../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)

---

## 1. Executive Summary

WRAITH-RedOps is a comprehensive Adversary Emulation Platform designed for authorized Red Team engagements. It provides a secure, resilient Command and Control (C2) infrastructure that leverages the WRAITH protocol's intrinsic stealth capabilities.

The platform consists of three primary components:
1.  **Team Server:** A multi-user collaboration hub managing state and tasking.
2.  **Operator Client:** A cross-platform GUI for campaign management.
3.  **Spectre Implant:** A modular, memory-resident agent ("Beacon") designed for stealth and evasion.

**Authorized Use Cases Only:**
- Executive-authorized red team exercises.
- Purple team collaborative assessments.
- Adversary emulation with defined objectives.

---

## 2. System Architecture

### 2.1 Component Topology

```mermaid
graph TD
    subgraph "Operator Network (Safe Zone)"
        Client[Operator Client (Tauri)]
        Dev[DevOps / Builder]
    end

    subgraph "C2 Infrastructure (Cloud/Redirectors)"
        TS[Team Server (PostgreSQL + WRAITH)]
        Red_UDP[UDP Redirector]
        Red_HTTP[HTTPS Redirector]
        Red_DNS[DNS Redirector]
    end

    subgraph "Target Network (Compromised)"
        Beacon_A[Spectre Implant (Gateway)]
        Beacon_B[Spectre Implant (SMB Peer)]
        Beacon_C[Spectre Implant (TCP Peer)]
    end

    Client <-->|gRPC/TLS| TS
    Dev -->|Build Artifacts| TS
    
    TS <-->|WRAITH Tunnel| Red_UDP
    TS <-->|HTTPS Tunnel| Red_HTTP
    
    Red_UDP <-->|WRAITH/UDP| Beacon_A
    Red_HTTP <-->|HTTPS| Beacon_A
    
    Beacon_A <-->|SMB Pipe| Beacon_B
    Beacon_B <-->|TCP Socket| Beacon_C
```

### 2.2 Component Descriptions

#### A. Operator Console (Client)
*   **Purpose:** Centralized management interface for red team operators.
*   **UI:** Tauri (Rust backend) + React (Frontend).
*   **Capabilities:**
    *   **Session Management:** Real-time interactive terminal for each beacon.
    *   **Graph View:** Visualizes the peer-to-peer graph of beacons.
    *   **Campaign Management:** Organization of engagement activities.

#### B. Team Server (Backend)
*   **Purpose:** The brain of the operation. Manages state, tasking, and data aggregation.
*   **Architecture:** Rust (`axum`) with PostgreSQL.
*   **Listener Bus:** Manages multiple listening ports (UDP, TCP, HTTP) and routes traffic to specific sessions.
*   **Builder:** Compiles unique implant artifacts per campaign using a patched LLVM toolchain.

#### C. "Spectre" Implant (Agent)
*   **Purpose:** The deployed agent executing on target systems.
*   **Design:** `no_std` Rust binary (freestanding). Zero runtime dependencies (no libc/msvcrt).
*   **Memory Model:** Position Independent Code (PIC). Can be injected as Shellcode (sRDI), DLL, or EXE.
*   **Stealth Features:**
    *   **Sleep Mask:** Obfuscates memory during sleep intervals.
    *   **Stack Spoofing:** Rewrites call stack frames to look legitimate.
    *   **Indirect Syscalls:** Bypasses user-mode hooks (EDR).

#### D. Governance Layer
*   **Purpose:** Enforce engagement parameters and maintain accountability.
*   **Controls:**
    *   **Scope Enforcement:** Target whitelist/blacklist checks kernel-side in the implant.
    *   **Time-to-Live (TTL):** Implants self-destruct after a specific date.
    *   **Audit Logging:** Immutable logs of every command sent.

---

## 3. Operational Workflow

```
┌──────────────────────────────────────────────────────────────────────────┐
│                      Red Team Engagement Workflow                        │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ PHASE 1: Pre-Engagement                                          │    │
│  │                                                                  │    │
│  │  • Authorization acquisition (executive sign-off)                │    │
│  │  • Scope definition and documentation                            │    │
│  │  • Infrastructure preparation (Redirectors, C2 Domains)          │    │
│  │  • Payload Generation (Builder)                                  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                  │                                      │
│                                  ▼                                      │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ PHASE 2: Operations                                              │    │
│  │                                                                  │    │
│  │  • Access: Initial Access vectors (Phishing, Exploit)            │    │
│  │  • Establish: Beacon check-in, key exchange                      │    │
│  │  • Persistence: Maintain access across restarts                  │    │
│  │  • Lateral Movement: SMB/TCP Peer-to-Peer chaining               │    │
│  │  • Objectives: Data staging, exfiltration (via WRAITH-Recon)     │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                  │                                      │
│                                  ▼                                      │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ PHASE 3: Post-Engagement                                         │    │
│  │                                                                  │    │
│  │  • Operations cessation                                          │    │
│  │  • Cleanup: Remove artifacts, revoke keys                        │    │
│  │  • Reporting: Generate Timeline, Finding Report                  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 4. C2 Protocol Specification

### 4.1 Transport Layer
*   **Primary:** WRAITH Protocol (UDP/Noise_XX). Provides encryption, authentication, and NAT traversal.
*   **Fallback:** HTTPS (TLS 1.3), DNS (DoH), SMB (Named Pipes).

### 4.2 Presentation Layer (C2 Payload)
Encapsulated *inside* the transport layer.
*   **Header:** `[Magic:4][SessionID:4][TaskID:4][Opcode:2][Length:4]`
*   **Payload:** Protobuf-serialized data (Commands or Results).
*   **Encryption:** Inner layer Chacha20-Poly1305 (Session Key) + Outer layer Transport Encryption.

### 4.3 Protocol Data Unit (PDU) Definitions
We use Google Protocol Buffers (proto3) for defining the C2 schema.

```protobuf
syntax = "proto3";

message BeaconTask {
    uint32 task_id = 1;
    CommandType command = 2;
    bytes arguments = 3; // Serialized args specific to command
}

enum CommandType {
    SLEEP = 0;
    SHELL = 1;
    UPLOAD = 2;
    DOWNLOAD = 3;
    EXECUTE_BOF = 4;
    INJECT = 5;
    EXIT = 99;
}

message BeaconResponse {
    uint32 task_id = 1;
    uint32 status_code = 2; // 0 = Success
    bytes output = 3;
    string error_msg = 4;
}
```

---

## 5. Data Structures & Schema

### 5.1 Team Server Database (PostgreSQL)

```sql
CREATE TABLE listeners (
    id SERIAL PRIMARY KEY,
    name VARCHAR(64) UNIQUE NOT NULL,
    type VARCHAR(16) NOT NULL, -- UDP, HTTP, SMB
    bind_address INET NOT NULL,
    config JSONB NOT NULL
);

CREATE TABLE beacons (
    id CHAR(16) PRIMARY KEY, -- Random Hex ID
    internal_ip INET,
    external_ip INET,
    hostname VARCHAR(255),
    user_name VARCHAR(255),
    process_id INT,
    arch VARCHAR(8), -- x64, x86
    linked_beacon_id CHAR(16) REFERENCES beacons(id), -- Parent for P2P
    last_seen TIMESTAMP WITH TIME ZONE,
    status VARCHAR(16) -- ALIVE, DEAD, EXITING
);

CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    beacon_id CHAR(16) REFERENCES beacons(id),
    command_type INT NOT NULL,
    arguments BYTEA,
    queued_at TIMESTAMP DEFAULT NOW(),
    sent_at TIMESTAMP,
    completed_at TIMESTAMP,
    result_output BYTEA,
    operator_id INT REFERENCES users(id)
);
```

---

## 6. Detection Considerations

To support defensive improvement, WRAITH-RedOps produces detectable artifacts:

### Network Indicators
| Indicator Type | Description | Detection Approach |
|----------------|-------------|-------------------|
| **Beaconing** | Periodic communications | Interval analysis / Jitter analysis |
| **Data Patterns** | Unusual traffic volumes | Baseline deviation |
| **Certificate Analysis** | TLS certificate properties | Certificate transparency |

### Endpoint Indicators
| Indicator Type | Description | Detection Approach |
|----------------|-------------|-------------------|
| **Process Ancestry** | Unusual process relationships | EDR process tracking |
| **Memory Indicators** | Unbacked RWX pages (if sleep mask fails) | Memory scanning (Moneta) |
| **Named Pipes** | Abnormal pipe names (e.g., `\pipe\msagent_12`) | Sysmon Event ID 17/18 |

---

## 7. Audit and Accountability

### 7.1 Logging Requirements
| Log Category | Contents | Integrity |
|--------------|----------|-----------|
| **Operator Log** | All operator actions | Signed, attributed |
| **Communications Log** | Channel activity | Cryptographic chain |
| **Operations Log** | Task execution details | Append-only, encrypted |

### 7.2 Chain of Custody
1.  **Executive Authorization:** Signed authorization blob.
2.  **WRAITH-RedOps Platform:** Enforces constraints, logs all activity.
3.  **Audit Trail:** Available for review, incident response, legal.

---

## 8. Deployment Considerations

### Prerequisites
*   **Signed Rules of Engagement document.**
*   **Scope configuration file.**
*   **Operator credentials.**
*   **Kill switch endpoint configuration.**

### Infrastructure Requirements
*   **Team Server:** Hardened Linux VPS (4 vCPU, 8GB RAM).
*   **Redirectors:** Ephemeral VPS instances (dumb pipes).
*   **Domains:** Categorized/Aged domains for HTTP/DNS C2.