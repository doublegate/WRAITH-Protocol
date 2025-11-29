# WRAITH-Recon Reference Architecture

**Document Version:** 1.0.0
**Last Updated:** 2025-11-29
**Classification:** Reference Architecture (High-Level)
**Governance:** See [Security Testing Parameters](../../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)

---

## Executive Summary

WRAITH-Recon is a reconnaissance and data transfer assessment tool designed for authorized security testing engagements. It leverages WRAITH Protocol capabilities to evaluate an organization's detection and prevention controls for data movement across network boundaries.

**Authorized Use Cases Only:**
- Contracted penetration testing engagements
- Red team exercises with executive authorization
- CTF competitions with explicit tool permissions
- Security research in isolated laboratory environments

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        WRAITH-Recon Architecture                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐     │
│  │   Recon Module  │    │  Transfer Module │    │ Governance Module│    │
│  │                 │    │                 │    │                 │     │
│  │ - Asset Enum    │    │ - Staging       │    │ - Scope Enforce │     │
│  │ - Service Map   │    │ - Chunking      │    │ - Time Limits   │     │
│  │ - Path Analysis │    │ - Channel Mgmt  │    │ - Audit Logging │     │
│  └────────┬────────┘    └────────┬────────┘    └────────┬────────┘     │
│           │                      │                      │               │
│           └──────────────────────┼──────────────────────┘               │
│                                  │                                      │
│                    ┌─────────────▼─────────────┐                        │
│                    │    WRAITH Protocol Stack   │                        │
│                    │                           │                        │
│                    │  wraith-transport         │                        │
│                    │  wraith-crypto            │                        │
│                    │  wraith-obfuscation       │                        │
│                    │  wraith-discovery         │                        │
│                    └───────────────────────────┘                        │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Component Descriptions

### 1. Reconnaissance Module

**Purpose:** Enumerate authorized target assets and map network topology within engagement scope.

**Capability Categories:**

| Category | Description | Control Tested |
|----------|-------------|----------------|
| Asset Enumeration | Identify systems within authorized scope | Asset inventory accuracy |
| Service Mapping | Discover services and their configurations | Service hardening |
| Path Analysis | Identify potential data movement paths | Network segmentation |
| Credential Discovery | Locate credential artifacts (authorized scope) | Credential hygiene |

**Integration Points:**
- Consumes scope configuration from Governance Module
- Outputs findings to Transfer Module for path selection
- All activities logged to audit subsystem

### 2. Transfer Module

**Purpose:** Evaluate data movement controls by attempting authorized transfers through various channels.

**Capability Categories:**

| Category | Description | Control Tested |
|----------|-------------|----------------|
| Data Staging | Aggregate test data in memory | Endpoint monitoring |
| Chunking | Fragment data for transmission | DLP reassembly |
| Channel Management | Select and manage transfer paths | Egress filtering |
| Protocol Selection | Choose appropriate WRAITH transport mode | Protocol inspection |

**WRAITH Protocol Integration:**
- Uses `wraith-transport` for channel establishment
- Uses `wraith-crypto` for payload protection
- Uses `wraith-obfuscation` for traffic pattern testing
- Uses `wraith-files` for chunking operations

### 3. Governance Module

**Purpose:** Enforce engagement boundaries and maintain audit trail.

**Capability Categories:**

| Category | Description | Governance Function |
|----------|-------------|---------------------|
| Scope Enforcement | Restrict operations to authorized targets | Prevent scope creep |
| Time Boundaries | Limit operations to engagement window | Temporal control |
| Kill Switch | Enable immediate operation termination | Emergency stop |
| Audit Logging | Record all operations with integrity | Accountability |
| Data Handling | Enforce synthetic data usage | Data protection |

**Configuration Reference:**
```
# Conceptual scope configuration
[engagement]
id = "ENG-2025-001"
start = "2025-12-01T00:00:00Z"
end = "2025-12-15T23:59:59Z"

[scope.allowed]
networks = ["10.0.0.0/8", "192.168.1.0/24"]
domains = ["*.target.local"]

[scope.denied]
networks = ["10.0.1.0/24"]  # Critical systems
keywords = ["PRODUCTION", "PII"]

[governance]
kill_switch = true
synthetic_data_only = true
log_encryption = true
```

---

## Operational Workflow

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        Operational Workflow                              │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  [1. Authorization]                                                      │
│         │                                                                │
│         │  Load signed RoE, verify engagement parameters                 │
│         ▼                                                                │
│  [2. Scope Validation]                                                   │
│         │                                                                │
│         │  Configure target whitelist, time boundaries                   │
│         ▼                                                                │
│  [3. Reconnaissance]                                                     │
│         │                                                                │
│         │  Enumerate assets, map services, identify paths                │
│         ▼                                                                │
│  [4. Path Selection]                                                     │
│         │                                                                │
│         │  Choose transfer channels based on test objectives             │
│         ▼                                                                │
│  [5. Transfer Execution]                                                 │
│         │                                                                │
│         │  Attempt data movement via selected channels                   │
│         ▼                                                                │
│  [6. Result Collection]                                                  │
│         │                                                                │
│         │  Record success/failure, control effectiveness                 │
│         ▼                                                                │
│  [7. Reporting]                                                          │
│         │                                                                │
│         │  Generate findings, recommendations, audit log                 │
│         ▼                                                                │
│  [8. Cleanup]                                                            │
│         │                                                                │
│         └──▶  Remove artifacts, verify clean state                       │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## WRAITH Protocol Integration

### Transport Layer Usage

WRAITH-Recon leverages the protocol stack for transfer channel establishment:

| WRAITH Component | Recon Usage | Test Objective |
|------------------|-------------|----------------|
| `wraith-transport` | Channel establishment | Egress path validation |
| `wraith-crypto` | Payload encryption | DLP encrypted content handling |
| `wraith-obfuscation` | Traffic shaping | DPI effectiveness |
| `wraith-discovery` | Peer location | Network visibility |

### Protocol Modes

The tool can operate in various protocol modes to test different control types:

- **Direct Mode:** Standard WRAITH transport (baseline)
- **Mimicry Mode:** Protocol-shaped traffic (DPI testing)
- **Fragmented Mode:** Distributed transfer (reassembly testing)
- **Timed Mode:** Scheduled transmissions (behavioral detection)

---

## Detection Considerations

To support defensive improvement, WRAITH-Recon operations produce detectable artifacts:

### Network Indicators

| Indicator Type | Description | Detection Approach |
|----------------|-------------|-------------------|
| Connection Patterns | Periodic beaconing intervals | Statistical analysis |
| Traffic Volume | Unusual egress data volumes | Baseline deviation |
| Destination Analysis | Connections to uncommon endpoints | Reputation/geolocation |
| Protocol Anomalies | Mismatched protocol behaviors | Deep packet inspection |

### Endpoint Indicators

| Indicator Type | Description | Detection Approach |
|----------------|-------------|-------------------|
| Process Behavior | Unusual process network activity | EDR behavioral rules |
| File Operations | Staging area creation/access | File integrity monitoring |
| Memory Patterns | Data aggregation in memory | Memory analysis |

### Recommended Detection Development

Post-engagement, findings should inform:
- SIEM correlation rule development
- EDR behavioral signature creation
- Network detection rule tuning
- DLP policy refinement

---

## Audit and Accountability

### Logging Requirements

All operations must produce tamper-evident logs:

| Log Category | Contents | Integrity |
|--------------|----------|-----------|
| Command Log | All operator commands | Signed, timestamped |
| Network Log | Connections, transfers | Cryptographic chain |
| Finding Log | Discovered paths, results | Append-only |
| Error Log | Failures, blocked attempts | Complete capture |

### Report Deliverables

Standard engagement outputs:

1. **Executive Summary:** High-level findings, risk assessment
2. **Technical Report:** Detailed paths discovered, controls bypassed
3. **Detection Gap Analysis:** What should have been detected but wasn't
4. **Remediation Recommendations:** Specific control improvements
5. **Audit Log Package:** Complete operation record

---

## Deployment Considerations

### Prerequisites

- Signed Rules of Engagement document
- Scope configuration file (engagement-specific)
- Operator credentials with audit attribution
- Kill switch endpoint configuration
- Secure log collection endpoint

### Infrastructure Requirements

- Isolated operator workstation
- Secure communications channel to engagement lead
- Log aggregation with integrity verification
- Emergency contact procedures

---

## Relationship to Other Clients

WRAITH-Recon focuses specifically on reconnaissance and data transfer assessment:

| Client | Relationship |
|--------|--------------|
| WRAITH-Transfer | Shares chunking/transfer primitives |
| WRAITH-RedOps | May operate in conjunction for full-scope testing |
| WRAITH-Share | Uses similar DHT discovery patterns |

---

## Compliance Alignment

Operations should align with:

- **PTES:** Pre-engagement, intelligence gathering, exploitation, post-exploitation, reporting
- **NIST SP 800-115:** Technical security testing guidance
- **MITRE ATT&CK:** Reconnaissance, Collection, Exfiltration tactics
- **PCI-DSS:** Penetration testing requirements (if applicable)

---

## References

- [Security Testing Parameters](../../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)
- [WRAITH Protocol Overview](../../architecture/protocol-overview.md)
- [Client Overview](../overview.md)

---

*This document describes reference architecture only. Implementation requires appropriate authorization, operator qualifications, and adherence to the governance framework.*
