# WRAITH-RedOps Reference Architecture

**Document Version:** 1.0.0
**Last Updated:** 2025-11-29
**Classification:** Reference Architecture (High-Level)
**Governance:** See [Security Testing Parameters](../../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)

---

## Executive Summary

WRAITH-RedOps is a red team operations platform designed for authorized adversary emulation exercises. It provides capabilities for simulating advanced threat actor behaviors to evaluate an organization's detection, response, and resilience capabilities.

**Authorized Use Cases Only:**
- Executive-authorized red team exercises
- Purple team collaborative assessments
- Adversary emulation with defined objectives
- CTF competitions with explicit permissions
- Security research in isolated environments

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       WRAITH-RedOps Architecture                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                      Operator Console                             │  │
│  │   Campaign Mgmt | Session Mgmt | Reporting | Kill Switch          │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                  │                                      │
│                    ┌─────────────▼─────────────┐                        │
│                    │    Governance Layer        │                        │
│                    │                           │                        │
│                    │  Scope | Time | Audit     │                        │
│                    └─────────────┬─────────────┘                        │
│                                  │                                      │
│         ┌────────────────────────┼────────────────────────┐             │
│         │                        │                        │             │
│         ▼                        ▼                        ▼             │
│  ┌─────────────┐         ┌─────────────┐         ┌─────────────┐       │
│  │ Communications│        │ Operations  │         │ Infrastructure│     │
│  │    Module    │         │   Module    │         │    Module    │       │
│  │             │         │             │         │             │       │
│  │ - Channels  │         │ - Tasks     │         │ - Redirectors│       │
│  │ - Protocols │         │ - Modules   │         │ - Relays     │       │
│  │ - Fallback  │         │ - Scripts   │         │ - Lifecycle  │       │
│  └──────┬──────┘         └──────┬──────┘         └──────┬──────┘       │
│         │                       │                       │               │
│         └───────────────────────┼───────────────────────┘               │
│                                 │                                       │
│                   ┌─────────────▼─────────────┐                         │
│                   │    WRAITH Protocol Stack   │                         │
│                   │                           │                         │
│                   │  wraith-transport         │                         │
│                   │  wraith-crypto            │                         │
│                   │  wraith-obfuscation       │                         │
│                   │  wraith-discovery         │                         │
│                   └───────────────────────────┘                         │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Component Descriptions

### 1. Operator Console

**Purpose:** Centralized management interface for red team operators.

**Capability Categories:**

| Category | Description | Function |
|----------|-------------|----------|
| Campaign Management | Organize engagement activities | Operational tracking |
| Session Management | Monitor active operations | Real-time visibility |
| Task Orchestration | Coordinate activities | Workflow management |
| Reporting | Generate engagement documentation | Deliverable creation |
| Kill Switch | Immediate operation termination | Emergency control |

### 2. Governance Layer

**Purpose:** Enforce engagement parameters and maintain accountability.

**Capability Categories:**

| Category | Description | Control |
|----------|-------------|---------|
| Scope Enforcement | Target whitelist/blacklist | Prevent scope violations |
| Time Boundaries | Engagement window limits | Temporal control |
| Audit Logging | Comprehensive activity recording | Accountability |
| Data Controls | Handling restrictions | Data protection |
| Authorization Verification | Credential/permission checks | Access control |

**Configuration Reference:**
```
# Conceptual engagement configuration
[engagement]
id = "REDTEAM-2025-Q4"
type = "adversary_emulation"
emulated_threat = "APT29"
start = "2025-12-01T00:00:00Z"
end = "2025-12-31T23:59:59Z"

[scope.authorized]
networks = ["10.0.0.0/8"]
domains = ["*.corp.target.com"]
objectives = ["credential_access", "lateral_movement", "data_staging"]

[scope.excluded]
networks = ["10.0.99.0/24"]  # Safety systems
systems = ["DC01", "BACKUP-*"]
actions = ["destructive", "dos"]

[governance]
deconfliction_contact = "soc-lead@target.com"
check_in_interval = "4h"
kill_switch_enabled = true
implant_expiry = "2026-01-02T00:00:00Z"
```

### 3. Communications Module

**Purpose:** Manage operator-to-operation communications channels.

**Capability Categories:**

| Category | Description | Test Objective |
|----------|-------------|----------------|
| Channel Establishment | Create communication paths | Network visibility |
| Protocol Handling | Support multiple protocols | Protocol inspection |
| Fallback Logic | Alternate channel activation | Resilience testing |
| Traffic Management | Control communication patterns | Behavioral detection |

**WRAITH Protocol Integration:**
- Uses `wraith-transport` for channel establishment
- Uses `wraith-crypto` for communication encryption
- Uses `wraith-obfuscation` for traffic pattern control
- Uses `wraith-discovery` for peer coordination

### 4. Operations Module

**Purpose:** Execute authorized testing activities within scope.

**Capability Categories:**

| Category | Description | Test Objective |
|----------|-------------|----------------|
| Task Execution | Run authorized operations | Control effectiveness |
| Module System | Extensible capability framework | Testing flexibility |
| Result Collection | Gather operation outcomes | Finding documentation |
| State Management | Track operational state | Continuity |

**Operational Domains (MITRE ATT&CK Aligned):**

| Domain | Description | Detection Focus |
|--------|-------------|-----------------|
| Initial Access | Entry point establishment | Perimeter controls |
| Persistence | Maintain access across restarts | Endpoint monitoring |
| Privilege Escalation | Elevate access levels | Access controls |
| Defense Evasion | Test detection coverage | Security tool gaps |
| Credential Access | Credential hygiene assessment | Authentication security |
| Discovery | Environment enumeration | Internal visibility |
| Lateral Movement | Cross-system access | Segmentation |
| Collection | Data aggregation | DLP effectiveness |
| Exfiltration | Data transfer (via WRAITH-Recon) | Egress controls |

### 5. Infrastructure Module

**Purpose:** Manage supporting infrastructure for operations.

**Capability Categories:**

| Category | Description | Function |
|----------|-------------|----------|
| Redirector Management | Traffic routing infrastructure | Attribution management |
| Relay Coordination | Multi-hop communication | Path diversity |
| Lifecycle Management | Infrastructure provisioning/teardown | Operational hygiene |
| Deconfliction | Blue team coordination (if purple) | Avoid interference |

---

## Operational Workflow

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
│  │  • Rules of Engagement finalization                              │    │
│  │  • Deconfliction procedures established                          │    │
│  │  • Infrastructure preparation                                    │    │
│  │  • Kill switch verification                                      │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                  │                                      │
│                                  ▼                                      │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ PHASE 2: Operations                                              │    │
│  │                                                                  │    │
│  │  • Threat emulation execution                                    │    │
│  │  • Objective pursuit within scope                                │    │
│  │  • Regular check-ins with engagement lead                        │    │
│  │  • Finding documentation in real-time                            │    │
│  │  • Scope adherence verification                                  │    │
│  │  • Abort criteria monitoring                                     │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                  │                                      │
│                                  ▼                                      │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ PHASE 3: Post-Engagement                                         │    │
│  │                                                                  │    │
│  │  • Operations cessation                                          │    │
│  │  • Access removal verification                                   │    │
│  │  • Infrastructure decommissioning                                │    │
│  │  • Artifact cleanup confirmation                                 │    │
│  │  • Finding compilation                                           │    │
│  │  • Report generation                                             │    │
│  │  • Debrief delivery                                              │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## WRAITH Protocol Integration

### Transport Layer Usage

| WRAITH Component | RedOps Usage | Test Objective |
|------------------|--------------|----------------|
| `wraith-transport` | Communications channels | Network detection |
| `wraith-crypto` | Encrypted communications | Inspection bypass |
| `wraith-obfuscation` | Traffic pattern control | Behavioral detection |
| `wraith-discovery` | Peer/relay location | Network visibility |

### Protocol Modes

Operations can utilize various protocol modes to test different detection capabilities:

- **Standard Mode:** Baseline encrypted transport
- **Mimicry Mode:** Traffic shaped to resemble legitimate protocols
- **Jitter Mode:** Randomized timing patterns
- **Slow Mode:** Extended intervals for long-term operations
- **Burst Mode:** High-throughput for time-sensitive operations

---

## Detection Considerations

To support defensive improvement, WRAITH-RedOps produces detectable artifacts:

### Network Indicators

| Indicator Type | Description | Detection Approach |
|----------------|-------------|-------------------|
| Beaconing | Periodic communications | Interval analysis |
| Data Patterns | Unusual traffic volumes | Baseline deviation |
| Protocol Anomalies | Behavioral mismatches | Deep inspection |
| Destination Analysis | Uncommon endpoints | Reputation systems |
| Certificate Analysis | TLS certificate properties | Certificate transparency |

### Endpoint Indicators

| Indicator Type | Description | Detection Approach |
|----------------|-------------|-------------------|
| Process Ancestry | Unusual process relationships | EDR process tracking |
| Network Connections | Process network behavior | Connection monitoring |
| File System Activity | Unusual file operations | File integrity monitoring |
| Registry/Config Changes | Persistence artifacts | Configuration monitoring |
| Memory Indicators | In-memory patterns | Memory scanning |
| Scheduled Tasks | Persistence mechanisms | Task auditing |

### Post-Engagement Detection Development

Findings should inform:
- SIEM detection rule development
- EDR behavioral signature creation
- Network detection rule enhancement
- Threat hunting hypothesis development
- Security control tuning

---

## Audit and Accountability

### Logging Requirements

| Log Category | Contents | Integrity |
|--------------|----------|-----------|
| Operator Log | All operator actions | Signed, attributed |
| Communications Log | Channel activity | Cryptographic chain |
| Operations Log | Task execution details | Append-only, encrypted |
| Finding Log | Discovered vulnerabilities | Timestamped |
| Deconfliction Log | Blue team interactions | Complete record |

### Chain of Custody

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Accountability Chain                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  [Executive Authorization]                                              │
│         │                                                               │
│         │  Signed authorization, objectives, scope                      │
│         ▼                                                               │
│  [Engagement Lead]                                                      │
│         │                                                               │
│         │  Operational oversight, scope compliance                      │
│         ▼                                                               │
│  [Red Team Operators]                                                   │
│         │                                                               │
│         │  Execute within authorized scope                              │
│         ▼                                                               │
│  [WRAITH-RedOps Platform]                                               │
│         │                                                               │
│         │  Enforce constraints, log all activity                        │
│         ▼                                                               │
│  [Audit Trail]                                                          │
│         │                                                               │
│         └──▶ Available for review, incident response, legal            │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Report Deliverables

Standard engagement outputs:

| Deliverable | Description | Audience |
|-------------|-------------|----------|
| Executive Summary | Risk-focused high-level findings | Leadership |
| Technical Report | Detailed attack paths, TTPs used | Security team |
| Detection Gap Analysis | What was/wasn't detected | SOC/Blue team |
| TTP Matrix | MITRE ATT&CK mapping | Threat intelligence |
| Remediation Roadmap | Prioritized improvements | Security team |
| Audit Log Package | Complete operation record | Compliance/legal |

---

## Safety Controls

### Mandatory Safeguards

| Control | Description | Purpose |
|---------|-------------|---------|
| Scope Whitelist | Hard-coded target restrictions | Prevent scope violations |
| Time Expiry | Automatic operation cessation | Temporal boundaries |
| Kill Switch | Immediate termination capability | Emergency control |
| Artifact Cleanup | Removal procedures and verification | Clean exit |
| Deconfliction | Blue team communication channels | Avoid disruption |

### Abort Criteria

Operations must cease immediately upon:
- Scope boundary violation detected
- Unintended system impact observed
- Kill switch activation
- Emergency contact request
- Time boundary expiration
- Authorization revocation

---

## Relationship to Other Clients

| Client | Relationship |
|--------|--------------|
| WRAITH-Recon | Complements with reconnaissance/exfiltration focus |
| WRAITH-Transfer | Shares transport primitives |
| WRAITH-Chat | May use for operator coordination (authorized) |

---

## Compliance Alignment

Operations should align with:

- **PTES:** Full penetration testing methodology
- **NIST SP 800-115:** Technical security testing guidance
- **MITRE ATT&CK:** Adversary TTP framework
- **TIBER-EU:** Threat intelligence-based ethical red teaming (if applicable)
- **CBEST:** UK financial sector testing framework (if applicable)

---

## Operator Qualifications

Recommended qualifications for platform operators:

| Category | Examples |
|----------|----------|
| Certifications | OSCP, OSCE, OSEP, CRTO, GPEN, GXPN |
| Experience | Red team operations, penetration testing |
| Knowledge | Adversary TTPs, detection mechanisms |
| Ethics | Commitment to authorized testing only |

---

## References

- [Security Testing Parameters](../../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)
- [WRAITH-Recon Architecture](../wraith-recon/architecture.md)
- [WRAITH Protocol Overview](../../architecture/protocol-overview.md)
- [Client Overview](../overview.md)

---

*This document describes reference architecture only. Implementation requires executive authorization, qualified operators, and strict adherence to the governance framework. All operations must comply with applicable laws and regulations.*
