# WRAITH-RedOps Client - Sprint Planning (Granular)

**Client Name:** WRAITH-RedOps
**Tier:** 3 (Advanced)
**Timeline:** 16 weeks (4 sprints x 4 weeks)
**Total Story Points:** 240
**Protocol Alignment:** Synchronized with core protocol development (Phases 1-6)
**Governance:** [Security Testing Parameters](../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)

---

## WRAITH Protocol Stack Dependencies

| Crate | Purpose | Integration Phase |
|-------|---------|-------------------|
| wraith-core | Frame construction, session management, stream multiplexing | Phase 1-2 |
| wraith-crypto | Noise_XX handshake, AEAD encryption, Elligator2, Double Ratchet | Phase 1-2 |
| wraith-transport | AF_XDP, io_uring, multi-path routing | Phase 2-3 |
| wraith-obfuscation | Padding, timing jitter, protocol mimicry (TLS 1.3, DNS, DoH) | Phase 2-3 |
| wraith-discovery | DHT integration, relay coordination, NAT traversal | Phase 3-4 |
| wraith-files | Chunking, BLAKE3 integrity, compression, streaming | Phase 3-4 |

## Protocol Integration Points

### Cryptographic Suite (wraith-crypto)
- **AEAD:** XChaCha20-Poly1305 (192-bit nonce, 256-bit key)
- **Key Exchange:** X25519 + Elligator2 encoding
- **Handshake:** Noise_XX (mutual authentication, identity hiding)
- **Ratcheting:** Double Ratchet with symmetric + DH ratchets
- **Hash:** BLAKE3 for integrity and KDF

### Wire Format (wraith-core)
- **Outer Packet:** 8B CID + encrypted payload + 16B auth tag
- **Inner Frame:** 28B header (type, stream_id, offset, length, flags) + payload + padding
- **Frame Types:** DATA, ACK, CONTROL, REKEY, PING/PONG, CLOSE, PAD, STREAM_*

### Transport Modes (wraith-transport)
- **AF_XDP:** 10-40 Gbps, kernel bypass, zero-copy
- **io_uring:** 1-5 Gbps, async I/O
- **UDP Fallback:** 300+ Mbps, broad compatibility

---

## MITRE ATT&CK Coverage Matrix

### Tactic: Initial Access (TA0001)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Phishing: Spearphishing Attachment | T1566.001 | Payload delivery simulation | REDOPS-003 |
| Supply Chain Compromise | T1195 | Update mechanism testing | REDOPS-004 |
| Valid Accounts | T1078 | Credential validation | REDOPS-005 |

### Tactic: Execution (TA0002)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Command and Scripting Interpreter | T1059 | Multi-language execution | REDOPS-006 |
| Native API | T1106 | Direct syscall interface | REDOPS-006 |
| Shared Modules | T1129 | DLL/SO injection | REDOPS-007 |

### Tactic: Persistence (TA0003)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Boot or Logon Autostart | T1547 | Registry/cron persistence | REDOPS-008 |
| Create Account | T1136 | Account creation testing | REDOPS-008 |
| Scheduled Task/Job | T1053 | Task scheduler abuse | REDOPS-008 |

### Tactic: Privilege Escalation (TA0004)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Exploitation for Privilege Escalation | T1068 | Exploit framework integration | REDOPS-009 |
| Process Injection | T1055 | Memory injection testing | REDOPS-007 |
| Valid Accounts: Domain Accounts | T1078.002 | Kerberos testing | REDOPS-010 |

### Tactic: Defense Evasion (TA0005)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Obfuscated Files or Information | T1027 | Payload encoding | REDOPS-011 |
| Indicator Removal | T1070 | Log/artifact cleanup | REDOPS-012 |
| Masquerading | T1036 | Process name spoofing | REDOPS-011 |
| Traffic Signaling | T1205 | Protocol triggering | REDOPS-002 |

### Tactic: Credential Access (TA0006)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| OS Credential Dumping | T1003 | LSASS/SAM extraction | REDOPS-013 |
| Brute Force | T1110 | Credential testing | REDOPS-014 |
| Credentials from Password Stores | T1555 | Browser/keychain access | REDOPS-013 |

### Tactic: Discovery (TA0007)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Network Service Discovery | T1046 | Port/service scanning | REDOPS-015 |
| System Information Discovery | T1082 | Host enumeration | REDOPS-015 |
| Domain Trust Discovery | T1482 | AD trust mapping | REDOPS-016 |

### Tactic: Lateral Movement (TA0008)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Remote Services | T1021 | SSH/RDP/WinRM | REDOPS-017 |
| Lateral Tool Transfer | T1570 | File staging | REDOPS-017 |
| Pass the Hash/Ticket | T1550 | Credential relay | REDOPS-018 |

### Tactic: Collection (TA0009)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Data from Local System | T1005 | File collection | REDOPS-019 |
| Screen Capture | T1113 | Display capture | REDOPS-019 |
| Keylogging | T1056.001 | Input capture | REDOPS-020 |

### Tactic: Command and Control (TA0011)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Application Layer Protocol | T1071 | HTTPS/DNS/DoH C2 | REDOPS-001 |
| Encrypted Channel | T1573 | Noise_XX channels | REDOPS-001 |
| Multi-Stage Channels | T1104 | Staged delivery | REDOPS-002 |
| Protocol Tunneling | T1572 | WRAITH tunneling | REDOPS-002 |

### Tactic: Exfiltration (TA0010)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Exfiltration Over C2 Channel | T1041 | Integrated exfil | REDOPS-021 |
| Exfiltration Over Alternative Protocol | T1048 | DNS/ICMP exfil | REDOPS-021 |
| Scheduled Transfer | T1029 | Timed exfiltration | REDOPS-022 |

### Tactic: Impact (TA0040)
| Technique | ID | Implementation | Story |
|-----------|-------|----------------|-------|
| Data Destruction | T1485 | Secure wipe testing | REDOPS-023 |
| Service Stop | T1489 | Service disruption | REDOPS-023 |
| Resource Hijacking | T1496 | Resource abuse simulation | REDOPS-024 |

---

## User Stories

### REDOPS-001: C2 Channel Framework

**As a** red team operator,
**I want** a flexible C2 channel framework using WRAITH protocol,
**So that** I can establish covert command and control channels.

**Story Points:** 34
**Priority:** P0 (Critical)
**Sprint:** Phase 1, Week 1-4
**MITRE ATT&CK:** T1071, T1573

#### Acceptance Criteria

1. Establish Noise_XX encrypted channels with mutual authentication
2. Support multiple transport protocols (HTTPS, DNS, DoH, raw UDP)
3. Implement beacon interval jitter with configurable distribution
4. Auto-rotate encryption keys using Double Ratchet
5. Survive network interruptions with session resumption

#### Implementation

```rust
//! REDOPS-001: C2 Channel Framework
//! Location: wraith-redops/src/c2/channel.rs

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use wraith_crypto::{NoiseSession, DoubleRatchet, Elligator2};
use wraith_obfuscation::TimingProfile;

/// C2 channel configuration
#[derive(Debug, Clone)]
pub struct C2Config {
    /// Primary transport protocol
    pub transport: TransportProtocol,

    /// Fallback transports (in priority order)
    pub fallbacks: Vec<TransportProtocol>,

    /// Beacon interval range (min, max) in seconds
    pub beacon_interval: (u64, u64),

    /// Jitter distribution
    pub jitter: JitterDistribution,

    /// Maximum retry attempts
    pub max_retries: u32,

    /// Session timeout
    pub session_timeout: Duration,

    /// Key ratchet interval (messages)
    pub ratchet_interval: u32,
}

#[derive(Debug, Clone)]
pub enum TransportProtocol {
    /// HTTPS with domain fronting
    Https {
        endpoint: String,
        fronting_domain: Option<String>,
        user_agent: String,
    },

    /// DNS over authoritative server
    Dns {
        resolver: String,
        domain: String,
        record_type: DnsRecordType,
    },

    /// DNS over HTTPS
    Doh {
        endpoint: String,
        domain: String,
    },

    /// Raw WRAITH protocol over UDP
    Wraith {
        endpoint: String,
        port: u16,
    },

    /// ICMP tunneling
    Icmp {
        target: String,
    },
}

#[derive(Debug, Clone)]
pub enum DnsRecordType {
    Txt,
    Cname,
    Mx,
    Null,
}

#[derive(Debug, Clone)]
pub enum JitterDistribution {
    /// Uniform random within range
    Uniform,
    /// Pareto distribution (more realistic)
    Pareto { alpha: f64 },
    /// Exponential distribution
    Exponential { lambda: f64 },
    /// No jitter (debugging only)
    None,
}

/// C2 channel state machine
pub struct C2Channel {
    config: C2Config,
    session: Arc<RwLock<Option<NoiseSession>>>,
    ratchet: Arc<RwLock<DoubleRatchet>>,
    state: Arc<RwLock<ChannelState>>,
    command_rx: mpsc::Receiver<Command>,
    response_tx: mpsc::Sender<Response>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelState {
    Disconnected,
    Connecting,
    Handshaking,
    Connected,
    Rekeying,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Command {
    pub id: u64,
    pub cmd_type: CommandType,
    pub payload: Vec<u8>,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum CommandType {
    Execute,
    Upload,
    Download,
    Enumerate,
    Persist,
    Cleanup,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct Response {
    pub command_id: u64,
    pub status: ResponseStatus,
    pub payload: Vec<u8>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Clone)]
pub enum ResponseStatus {
    Success,
    PartialSuccess,
    Failure(String),
    Timeout,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ResponseMetadata {
    pub duration_ms: u64,
    pub bytes_transferred: u64,
    pub retries: u32,
}

impl C2Channel {
    /// Create new C2 channel
    pub async fn new(
        config: C2Config,
    ) -> Result<(Self, mpsc::Sender<Command>, mpsc::Receiver<Response>), C2Error> {
        let (cmd_tx, command_rx) = mpsc::channel(100);
        let (response_tx, resp_rx) = mpsc::channel(100);

        let channel = Self {
            config,
            session: Arc::new(RwLock::new(None)),
            ratchet: Arc::new(RwLock::new(DoubleRatchet::new())),
            state: Arc::new(RwLock::new(ChannelState::Disconnected)),
            command_rx,
            response_tx,
        };

        Ok((channel, cmd_tx, resp_rx))
    }

    /// Calculate beacon interval with jitter
    fn calculate_beacon_interval(&self) -> Duration {
        let (min, max) = self.config.beacon_interval;
        let base = rand::random::<f64>() * (max - min) as f64 + min as f64;

        let jittered = match &self.config.jitter {
            JitterDistribution::Uniform => base,
            JitterDistribution::Pareto { alpha } => {
                let u: f64 = rand::random();
                base * (1.0 - u).powf(-1.0 / alpha)
            }
            JitterDistribution::Exponential { lambda } => {
                let u: f64 = rand::random();
                base - (1.0 / lambda) * u.ln()
            }
            JitterDistribution::None => base,
        };

        Duration::from_secs_f64(jittered.max(1.0))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum C2Error {
    #[error("Connection failed")]
    ConnectionFailed,
    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    #[error("Channel closed")]
    ChannelClosed,
    #[error("Timeout")]
    Timeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_beacon_interval_jitter() {
        let config = C2Config {
            transport: TransportProtocol::Wraith {
                endpoint: "127.0.0.1".to_string(),
                port: 9999,
            },
            fallbacks: vec![],
            beacon_interval: (30, 60),
            jitter: JitterDistribution::Pareto { alpha: 1.5 },
            max_retries: 3,
            session_timeout: Duration::from_secs(300),
            ratchet_interval: 100,
        };

        let (channel, _, _) = C2Channel::new(config).await.unwrap();

        for _ in 0..100 {
            let interval = channel.calculate_beacon_interval();
            assert!(interval >= Duration::from_secs(1));
        }
    }
}
```

#### Governance Checkpoint

- [x] All C2 traffic encrypted with Noise_XX
- [x] Beacon intervals comply with RoE limits
- [x] Kill switch integration verified
- [x] Network boundaries respected

---

### REDOPS-002: Multi-Stage Payload Delivery

**As a** red team operator,
**I want** multi-stage payload delivery,
**So that** I can bypass detection with staged execution.

**Story Points:** 21
**Priority:** P0 (Critical)
**Sprint:** Phase 1, Week 2-4
**MITRE ATT&CK:** T1104, T1572, T1205

#### Acceptance Criteria

1. Support 3+ delivery stages (dropper, loader, implant)
2. Each stage independently encrypted
3. Stage retrieval via multiple protocols
4. Anti-analysis checks between stages
5. Clean abort on detection

---

### REDOPS-003: Adversary Emulation Playbooks

**As a** security assessor,
**I want** pre-built adversary emulation playbooks,
**So that** I can simulate specific threat actors.

**Story Points:** 21
**Priority:** P1 (High)
**Sprint:** Phase 2, Week 5-8
**MITRE ATT&CK:** Multiple (per playbook)

#### APT29 (Cozy Bear) Playbook

```yaml
name: APT29 Emulation
description: Russian state-sponsored threat actor
techniques:
  initial_access:
    - technique: T1566.001  # Spearphishing Attachment
      implementation: Office macro delivery
    - technique: T1195.002  # Compromise Software Supply Chain
      implementation: Update hijacking

  execution:
    - technique: T1059.001  # PowerShell
      implementation: Encoded commands
    - technique: T1204.002  # Malicious File
      implementation: LNK execution

  persistence:
    - technique: T1547.001  # Registry Run Keys
      implementation: CurrentVersion\Run
    - technique: T1053.005  # Scheduled Task
      implementation: Daily beacon task

  defense_evasion:
    - technique: T1027.001  # Binary Padding
      implementation: Large PE padding
    - technique: T1070.004  # File Deletion
      implementation: Timestomp and delete

  credential_access:
    - technique: T1003.001  # LSASS Memory
      implementation: Memory extraction variant
    - technique: T1555.003  # Credentials from Web Browsers
      implementation: Chrome/Firefox extraction

  discovery:
    - technique: T1082  # System Information Discovery
      implementation: WMI queries
    - technique: T1016  # System Network Configuration
      implementation: ipconfig, netstat

  lateral_movement:
    - technique: T1021.002  # SMB/Windows Admin Shares
      implementation: Remote execution variant
    - technique: T1550.002  # Pass the Hash
      implementation: NTLM relay

  collection:
    - technique: T1005  # Data from Local System
      implementation: Document staging
    - technique: T1560.001  # Archive via Utility
      implementation: Encrypted archives

  exfiltration:
    - technique: T1041  # Exfiltration Over C2
      implementation: HTTPS chunked
    - technique: T1048.002  # Exfiltration Over Asymmetric Encrypted Non-C2
      implementation: Cloud storage API

indicators:
  network:
    - User-Agent pattern: Standard browser strings
    - C2 pattern: Cloud service domains
    - Beacon interval: 4-6 hours

  host:
    - Scheduled task naming conventions
    - Registry key patterns
    - File staging locations
```

#### APT28 (Fancy Bear) Playbook

```yaml
name: APT28 Emulation
description: Russian military intelligence operations
techniques:
  initial_access:
    - technique: T1566.002  # Spearphishing Link
      implementation: OAuth phishing
    - technique: T1190  # Exploit Public-Facing Application
      implementation: Web server vulnerabilities

  execution:
    - technique: T1059.003  # Windows Command Shell
      implementation: cmd.exe chains
    - technique: T1106  # Native API
      implementation: Direct syscalls

  persistence:
    - technique: T1547.009  # Shortcut Modification
      implementation: LNK hijacking
    - technique: T1136.001  # Local Account
      implementation: Hidden admin account

  privilege_escalation:
    - technique: T1068  # Exploitation for Privilege Escalation
      implementation: Kernel exploits
    - technique: T1055.001  # DLL Injection
      implementation: AppInit_DLLs

  defense_evasion:
    - technique: T1027.002  # Software Packing
      implementation: Custom packer
    - technique: T1562.001  # Disable Security Tools
      implementation: Service termination

  credential_access:
    - technique: T1110.003  # Password Spraying
      implementation: Low-and-slow spray
    - technique: T1558.003  # Kerberoasting
      implementation: Service ticket requests

  discovery:
    - technique: T1087.002  # Domain Account
      implementation: LDAP enumeration
    - technique: T1069.002  # Domain Groups
      implementation: Net group commands

  lateral_movement:
    - technique: T1021.001  # Remote Desktop Protocol
      implementation: RDP with stolen creds
    - technique: T1550.003  # Pass the Ticket
      implementation: Golden ticket

  collection:
    - technique: T1114.002  # Remote Email Collection
      implementation: EWS API access
    - technique: T1039  # Data from Network Shared Drive
      implementation: SMB enumeration

  command_and_control:
    - technique: T1071.001  # Web Protocols
      implementation: HTTP/S polling
    - technique: T1573.001  # Symmetric Cryptography
      implementation: Encrypted payloads

indicators:
  network:
    - C2 pattern: API-style endpoints
    - Certificate characteristics
    - DNS exfiltration patterns

  host:
    - Process injection patterns
    - WMI persistence methods
    - File staging conventions
```

---

### REDOPS-004: Implant Management System

**As a** red team lead,
**I want** a centralized implant management system,
**So that** I can coordinate multiple operators and implants.

**Story Points:** 34
**Priority:** P0 (Critical)
**Sprint:** Phase 2, Week 5-8

#### PostgreSQL Database Schema

```sql
-- REDOPS-004: Implant Management Database
-- Location: wraith-redops/migrations/001_initial.sql

-- Operators table
CREATE TABLE operators (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(64) UNIQUE NOT NULL,
    display_name VARCHAR(128),
    role VARCHAR(32) NOT NULL CHECK (role IN ('admin', 'operator', 'viewer')),
    public_key BYTEA NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_active TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT TRUE
);

-- Campaigns/Operations
CREATE TABLE campaigns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(128) NOT NULL,
    description TEXT,
    roe_document_id UUID,
    status VARCHAR(32) DEFAULT 'planning'
        CHECK (status IN ('planning', 'active', 'paused', 'completed', 'aborted')),
    start_date TIMESTAMPTZ,
    end_date TIMESTAMPTZ,
    created_by UUID REFERENCES operators(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Rules of Engagement documents
CREATE TABLE roe_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    campaign_id UUID REFERENCES campaigns(id),
    document JSONB NOT NULL,
    signature BYTEA NOT NULL,
    signing_key_id VARCHAR(64) NOT NULL,
    valid_from TIMESTAMPTZ NOT NULL,
    valid_until TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Implants
CREATE TABLE implants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    campaign_id UUID REFERENCES campaigns(id),
    hostname VARCHAR(255),
    internal_ip INET,
    external_ip INET,
    os_type VARCHAR(32),
    os_version VARCHAR(64),
    architecture VARCHAR(16),
    username VARCHAR(128),
    domain VARCHAR(255),
    privileges VARCHAR(32) CHECK (privileges IN ('user', 'admin', 'system')),
    implant_version VARCHAR(32),
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_checkin TIMESTAMPTZ,
    checkin_interval INTEGER,
    jitter_percent INTEGER,
    status VARCHAR(32) DEFAULT 'active'
        CHECK (status IN ('active', 'dormant', 'lost', 'killed')),
    notes TEXT,
    metadata JSONB
);

-- Implant network interfaces
CREATE TABLE implant_interfaces (
    id SERIAL PRIMARY KEY,
    implant_id UUID REFERENCES implants(id) ON DELETE CASCADE,
    interface_name VARCHAR(64),
    ip_address INET,
    mac_address MACADDR,
    is_primary BOOLEAN DEFAULT FALSE
);

-- Command queue
CREATE TABLE commands (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    implant_id UUID REFERENCES implants(id),
    operator_id UUID REFERENCES operators(id),
    command_type VARCHAR(64) NOT NULL,
    payload BYTEA,
    payload_encrypted BOOLEAN DEFAULT TRUE,
    priority INTEGER DEFAULT 5 CHECK (priority BETWEEN 1 AND 10),
    status VARCHAR(32) DEFAULT 'pending'
        CHECK (status IN ('pending', 'sent', 'received',
                          'executing', 'completed', 'failed', 'cancelled')),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    sent_at TIMESTAMPTZ,
    received_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    timeout_seconds INTEGER DEFAULT 300,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3
);

-- Command results
CREATE TABLE command_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    command_id UUID REFERENCES commands(id),
    output BYTEA,
    output_encrypted BOOLEAN DEFAULT TRUE,
    exit_code INTEGER,
    error_message TEXT,
    execution_time_ms INTEGER,
    received_at TIMESTAMPTZ DEFAULT NOW()
);

-- Collected files/artifacts
CREATE TABLE artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    implant_id UUID REFERENCES implants(id),
    command_id UUID REFERENCES commands(id),
    filename VARCHAR(512),
    original_path VARCHAR(1024),
    file_hash_sha256 BYTEA,
    file_hash_blake3 BYTEA,
    file_size BIGINT,
    mime_type VARCHAR(128),
    content BYTEA,
    collected_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB
);

-- Credentials
CREATE TABLE credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    implant_id UUID REFERENCES implants(id),
    source VARCHAR(64),
    credential_type VARCHAR(32)
        CHECK (credential_type IN ('password', 'hash', 'ticket', 'key', 'token')),
    domain VARCHAR(255),
    username VARCHAR(255),
    credential_data BYTEA,
    collected_at TIMESTAMPTZ DEFAULT NOW(),
    validated BOOLEAN,
    metadata JSONB
);

-- Activity log for audit
CREATE TABLE activity_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    operator_id UUID REFERENCES operators(id),
    implant_id UUID REFERENCES implants(id),
    campaign_id UUID REFERENCES campaigns(id),
    action VARCHAR(64) NOT NULL,
    details JSONB,
    source_ip INET,
    success BOOLEAN
);

-- Indexes for common queries
CREATE INDEX idx_implants_campaign ON implants(campaign_id);
CREATE INDEX idx_implants_status ON implants(status);
CREATE INDEX idx_implants_last_checkin ON implants(last_checkin);
CREATE INDEX idx_commands_implant ON commands(implant_id);
CREATE INDEX idx_commands_status ON commands(status);
CREATE INDEX idx_commands_created ON commands(created_at);
CREATE INDEX idx_activity_timestamp ON activity_log(timestamp);
CREATE INDEX idx_activity_campaign ON activity_log(campaign_id);
CREATE INDEX idx_credentials_domain_user ON credentials(domain, username);

-- Full-text search on activity details
CREATE INDEX idx_activity_details_gin ON activity_log USING GIN (details);
```

#### gRPC Protocol Definition

```protobuf
// REDOPS-004: Implant Management gRPC Protocol
// Location: wraith-redops/proto/redops.proto

syntax = "proto3";

package wraith.redops;

import "google/protobuf/timestamp.proto";
import "google/protobuf/empty.proto";

// Operator service for human interaction
service OperatorService {
    // Authentication
    rpc Authenticate(AuthRequest) returns (AuthResponse);
    rpc RefreshToken(RefreshRequest) returns (AuthResponse);

    // Campaign management
    rpc CreateCampaign(CreateCampaignRequest) returns (Campaign);
    rpc GetCampaign(GetCampaignRequest) returns (Campaign);
    rpc ListCampaigns(ListCampaignsRequest) returns (ListCampaignsResponse);
    rpc UpdateCampaign(UpdateCampaignRequest) returns (Campaign);

    // Implant management
    rpc ListImplants(ListImplantsRequest) returns (ListImplantsResponse);
    rpc GetImplant(GetImplantRequest) returns (Implant);
    rpc KillImplant(KillImplantRequest) returns (google.protobuf.Empty);

    // Command execution
    rpc SendCommand(SendCommandRequest) returns (Command);
    rpc GetCommandResult(GetCommandResultRequest) returns (CommandResult);
    rpc ListCommands(ListCommandsRequest) returns (ListCommandsResponse);
    rpc CancelCommand(CancelCommandRequest) returns (google.protobuf.Empty);

    // Real-time events
    rpc StreamEvents(StreamEventsRequest) returns (stream Event);

    // Artifacts
    rpc ListArtifacts(ListArtifactsRequest) returns (ListArtifactsResponse);
    rpc DownloadArtifact(DownloadArtifactRequest) returns (stream ArtifactChunk);

    // Credentials
    rpc ListCredentials(ListCredentialsRequest) returns (ListCredentialsResponse);
}

// Implant service (C2 protocol)
service ImplantService {
    // Registration and check-in
    rpc Register(RegisterRequest) returns (RegisterResponse);
    rpc CheckIn(CheckInRequest) returns (CheckInResponse);

    // Command retrieval and results
    rpc GetPendingCommands(GetPendingCommandsRequest) returns (stream Command);
    rpc SubmitResult(SubmitResultRequest) returns (google.protobuf.Empty);

    // File transfer
    rpc UploadArtifact(stream ArtifactChunk) returns (UploadArtifactResponse);
    rpc DownloadPayload(DownloadPayloadRequest) returns (stream PayloadChunk);
}

// Messages
message AuthRequest {
    string username = 1;
    bytes signature = 2;
    bytes challenge = 3;
}

message AuthResponse {
    string token = 1;
    google.protobuf.Timestamp expires_at = 2;
    Operator operator = 3;
}

message Operator {
    string id = 1;
    string username = 2;
    string display_name = 3;
    string role = 4;
    google.protobuf.Timestamp last_active = 5;
}

message Campaign {
    string id = 1;
    string name = 2;
    string description = 3;
    string status = 4;
    google.protobuf.Timestamp start_date = 5;
    google.protobuf.Timestamp end_date = 6;
    RoeDocument roe = 7;
    int32 implant_count = 8;
    int32 active_implant_count = 9;
}

message RoeDocument {
    string id = 1;
    bytes document_json = 2;
    bytes signature = 3;
    google.protobuf.Timestamp valid_from = 4;
    google.protobuf.Timestamp valid_until = 5;
}

message Implant {
    string id = 1;
    string campaign_id = 2;
    string hostname = 3;
    string internal_ip = 4;
    string external_ip = 5;
    string os_type = 6;
    string os_version = 7;
    string architecture = 8;
    string username = 9;
    string domain = 10;
    string privileges = 11;
    string implant_version = 12;
    google.protobuf.Timestamp first_seen = 13;
    google.protobuf.Timestamp last_checkin = 14;
    int32 checkin_interval = 15;
    int32 jitter_percent = 16;
    string status = 17;
    repeated NetworkInterface interfaces = 18;
    map<string, string> metadata = 19;
}

message NetworkInterface {
    string name = 1;
    string ip_address = 2;
    string mac_address = 3;
    bool is_primary = 4;
}

message Command {
    string id = 1;
    string implant_id = 2;
    string operator_id = 3;
    string command_type = 4;
    bytes payload = 5;
    int32 priority = 6;
    string status = 7;
    google.protobuf.Timestamp created_at = 8;
    int32 timeout_seconds = 9;
}

message CommandResult {
    string id = 1;
    string command_id = 2;
    bytes output = 3;
    int32 exit_code = 4;
    string error_message = 5;
    int32 execution_time_ms = 6;
    google.protobuf.Timestamp received_at = 7;
}

message Event {
    string id = 1;
    string type = 2;
    google.protobuf.Timestamp timestamp = 3;
    string campaign_id = 4;
    string implant_id = 5;
    map<string, string> data = 6;
}

message ArtifactChunk {
    string artifact_id = 1;
    int64 offset = 2;
    bytes data = 3;
    bool is_last = 4;
}

// Request/Response messages
message CreateCampaignRequest {
    string name = 1;
    string description = 2;
    bytes roe_document = 3;
    bytes roe_signature = 4;
}

message GetCampaignRequest {
    string id = 1;
}

message ListCampaignsRequest {
    string status_filter = 1;
    int32 page_size = 2;
    string page_token = 3;
}

message ListCampaignsResponse {
    repeated Campaign campaigns = 1;
    string next_page_token = 2;
}

message UpdateCampaignRequest {
    string id = 1;
    string name = 2;
    string description = 3;
    string status = 4;
}

message ListImplantsRequest {
    string campaign_id = 1;
    string status_filter = 2;
    int32 page_size = 3;
    string page_token = 4;
}

message ListImplantsResponse {
    repeated Implant implants = 1;
    string next_page_token = 2;
}

message GetImplantRequest {
    string id = 1;
}

message KillImplantRequest {
    string id = 1;
    bool clean_artifacts = 2;
}

message SendCommandRequest {
    string implant_id = 1;
    string command_type = 2;
    bytes payload = 3;
    int32 priority = 4;
    int32 timeout_seconds = 5;
}

message GetCommandResultRequest {
    string command_id = 1;
}

message ListCommandsRequest {
    string implant_id = 1;
    string status_filter = 2;
    int32 page_size = 3;
    string page_token = 4;
}

message ListCommandsResponse {
    repeated Command commands = 1;
    string next_page_token = 2;
}

message CancelCommandRequest {
    string command_id = 1;
}

message StreamEventsRequest {
    string campaign_id = 1;
    repeated string event_types = 2;
}

message ListArtifactsRequest {
    string implant_id = 1;
    string campaign_id = 2;
    int32 page_size = 3;
    string page_token = 4;
}

message ListArtifactsResponse {
    repeated Artifact artifacts = 1;
    string next_page_token = 2;
}

message Artifact {
    string id = 1;
    string implant_id = 2;
    string filename = 3;
    string original_path = 4;
    bytes hash_sha256 = 5;
    int64 file_size = 6;
    string mime_type = 7;
    google.protobuf.Timestamp collected_at = 8;
}

message DownloadArtifactRequest {
    string artifact_id = 1;
}

message ListCredentialsRequest {
    string campaign_id = 1;
    string implant_id = 2;
    string credential_type = 3;
    int32 page_size = 4;
    string page_token = 5;
}

message ListCredentialsResponse {
    repeated Credential credentials = 1;
    string next_page_token = 2;
}

message Credential {
    string id = 1;
    string implant_id = 2;
    string source = 3;
    string credential_type = 4;
    string domain = 5;
    string username = 6;
    google.protobuf.Timestamp collected_at = 7;
    bool validated = 8;
}

// Implant-side messages
message RegisterRequest {
    bytes encrypted_registration = 1;
    bytes ephemeral_public = 2;
}

message RegisterResponse {
    string implant_id = 1;
    bytes encrypted_config = 2;
    int32 checkin_interval = 3;
    int32 jitter_percent = 4;
}

message CheckInRequest {
    string implant_id = 1;
    bytes session_data = 2;
}

message CheckInResponse {
    bool has_commands = 1;
    int32 command_count = 2;
    int32 next_checkin_seconds = 3;
    bytes metadata = 4;
}

message GetPendingCommandsRequest {
    string implant_id = 1;
    int32 max_commands = 2;
}

message SubmitResultRequest {
    string command_id = 1;
    bytes encrypted_result = 2;
}

message UploadArtifactResponse {
    string artifact_id = 1;
    bool success = 2;
    string error = 3;
}

message DownloadPayloadRequest {
    string payload_id = 1;
    int64 offset = 2;
}

message PayloadChunk {
    bytes data = 1;
    int64 offset = 2;
    bool is_last = 3;
}

message RefreshRequest {
    string token = 1;
}
```

---

## Phase 1: Command Infrastructure (Weeks 1-4)

### S1.1: Team Server Core (25 pts)
- [x] Setup Async Rust Project (Axum/Tokio)
- [x] Implement Database migrations (Sqlx/Postgres)
- [x] Define gRPC Protos (c2.proto, admin.proto)
- [x] Implement Listener Trait and UDP Listener
- [x] Implement TaskQueue logic with priority support

### S1.2: Operator Client (25 pts)
- [x] Scaffold Tauri App (Vite + React + TS)
- [x] Implement Auth Logic (JWT + mTLS)
- [x] Create Session Grid Component
- [x] Integrate xterm.js for Beacon Console
- [x] Implement file upload/download manager UI

---

## Phase 2: The Implant Core (Weeks 5-8)

### S2.1: no_std Foundation (30 pts)
- [x] Create no_std crate layout
- [x] Implement PanicHandler (Abort/Loop)
- [x] Implement ApiResolver (Hash-based import resolution)
- [x] Implement MiniHeap allocator
- [x] Write Entry Point Assembly (Stack alignment)

### S2.2: WRAITH Integration (30 pts)
- [x] Port wraith-crypto to no_std
- [x] Implement socket layer via Syscalls
- [x] Implement C2 Loop (Poll -> Dispatch -> Sleep)
- [x] Implement Command Dispatcher

---

## Phase 3: Tradecraft & Evasion (Weeks 9-12)

### S3.1: Advanced Loader (35 pts)
- [x] Implement syscall resolver
- [x] Implement ROP Chain generator for Sleep Mask
- [x] Implement Stack Spoofing
- [x] Implement security bypass logic

### S3.2: Post-Exploitation Features (25 pts)
- [x] Implement COFF Loader (BOF support)
- [x] Implement SOCKS4a Server state machine
- [x] Implement File System VFS
- [x] Implement Token Manipulation

---

## Phase 4: Lateral Movement & Polish (Weeks 13-16)

### S4.1: Peer-to-Peer C2 (30 pts)
- [x] Implement Named Pipe Server/Client
- [x] Implement Routing Logic (Mesh forwarding)
- [x] Update Team Server Graph to render P2P links

### S4.2: Automation & Builder (40 pts)
- [x] Implement LLVM/LLD invocation logic
- [x] Implement Config Patcher
- [x] Implement Obfuscation Pass
- [x] Write scripting bindings
- [x] Perform Final Red Team Simulation (E2E)

---

## Sprint Summary

| Sprint | Weeks | Story Points | Key Deliverables |
|--------|-------|--------------|------------------|
| Phase 1 | 1-4 | 60 | C2 framework, multi-stage delivery, basic implant |
| Phase 2 | 5-8 | 60 | Management system, adversary playbooks, persistence |
| Phase 3 | 9-12 | 60 | Credential access, lateral movement, collection |
| Phase 4 | 13-16 | 60 | Exfiltration, evasion, TUI dashboard |

## Governance Gates

### Phase 1 Exit Criteria
- [x] Noise_XX encrypted C2 operational
- [x] Multi-stage delivery tested in lab
- [x] RoE enforcement active on all operations
- [x] Kill switch functional < 1ms

### Phase 2 Exit Criteria
- [x] PostgreSQL database deployed
- [x] gRPC API functional
- [x] APT29/APT28 playbooks tested
- [x] Operator authentication working

### Phase 3 Exit Criteria
- [x] Credential extraction tested
- [x] Lateral movement chains verified
- [x] All artifacts encrypted at rest
- [x] Audit logging complete

### Phase 4 Exit Criteria
- [x] Multi-path exfiltration operational
- [x] All MITRE ATT&CK techniques mapped
- [x] TUI dashboard complete
- [x] Full integration test passed

---

## Test Cases (20+)

### TC-001: C2 Channel Establishment
- **Precondition:** Team server running, valid RoE
- **Steps:** Initiate connection, complete handshake, send beacon
- **Expected:** Encrypted channel established, mutual auth verified

### TC-002: Kill Switch Activation
- **Precondition:** Active implant session
- **Steps:** Send HALT signal via UDP broadcast
- **Expected:** Implant terminates < 1ms, all operations cease

### TC-003: RoE Boundary Enforcement
- **Precondition:** RoE limits to 10.0.0.0/8
- **Steps:** Attempt action on 192.168.1.0/24
- **Expected:** Action blocked, violation logged

### TC-004: Multi-Stage Payload Delivery
- **Precondition:** Staged payloads configured
- **Steps:** Execute dropper, fetch loader, fetch implant
- **Expected:** Each stage independently encrypted, verified

### TC-005: Beacon Jitter Distribution
- **Precondition:** Pareto jitter configured
- **Steps:** Monitor 1000 beacon intervals
- **Expected:** Distribution matches Pareto alpha parameter

### TC-006: Transport Failover
- **Precondition:** Primary HTTPS, fallback DNS
- **Steps:** Block HTTPS, verify DNS failover
- **Expected:** Automatic failover < 30s

### TC-007: Key Ratchet Operation
- **Precondition:** Active encrypted session
- **Steps:** Send 100 messages, trigger ratchet
- **Expected:** New keys derived, old messages undecryptable

### TC-008: Implant Registration
- **Precondition:** Team server ready
- **Steps:** New implant registers with system info
- **Expected:** Implant appears in management system

### TC-009: Command Queue Priority
- **Precondition:** 10 commands queued with varying priority
- **Steps:** Implant checks in
- **Expected:** Commands delivered in priority order

### TC-010: Credential Collection
- **Precondition:** Implant with elevated privileges
- **Steps:** Execute credential dump command
- **Expected:** Credentials collected, encrypted, stored

### TC-011: Lateral Movement Chain
- **Precondition:** Valid credentials, target host
- **Steps:** Execute remote execution movement
- **Expected:** New implant on target, logged in audit

### TC-012: Artifact Upload
- **Precondition:** File collection command
- **Steps:** Collect file, upload chunked
- **Expected:** File stored encrypted, hash verified

### TC-013: DNS C2 Channel
- **Precondition:** DNS transport configured
- **Steps:** Establish DNS tunneling channel
- **Expected:** Bidirectional communication via TXT records

### TC-014: ICMP C2 Channel
- **Precondition:** ICMP transport configured
- **Steps:** Establish ICMP tunneling
- **Expected:** Data embedded in echo request/reply

### TC-015: APT29 Playbook Execution
- **Precondition:** Lab environment
- **Steps:** Execute full APT29 playbook
- **Expected:** All techniques execute in sequence

### TC-016: Operator Authentication
- **Precondition:** Operator credentials
- **Steps:** Authenticate with challenge-response
- **Expected:** Token issued, role enforced

### TC-017: Real-time Event Stream
- **Precondition:** gRPC stream open
- **Steps:** Implant checks in
- **Expected:** Event delivered to subscribed operators

### TC-018: Session Resumption
- **Precondition:** Previous session state saved
- **Steps:** Reconnect after network interruption
- **Expected:** Session resumes without re-handshake

### TC-019: Multi-Path Exfiltration
- **Precondition:** Multiple channels configured
- **Steps:** Exfiltrate large file
- **Expected:** Data split across channels, reassembled

### TC-020: Clean Abort Sequence
- **Precondition:** Active operation
- **Steps:** Trigger abort condition
- **Expected:** All artifacts removed, logs cleared (per RoE)

---

## Risk Register

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| C2 detection by EDR | High | Medium | Multiple evasion techniques, encrypted channels |
| RoE violation | Critical | Low | Multi-layer enforcement, audit logging |
| Key compromise | Critical | Low | Perfect forward secrecy, regular ratcheting |
| Implant instability | High | Medium | Comprehensive testing, graceful degradation |
| Network blocking | Medium | High | Transport fallbacks, domain fronting |

---

## Dependencies

### External Libraries
- `tokio` - Async runtime
- `tonic` - gRPC framework
- `sqlx` - PostgreSQL client
- `snow` - Noise Protocol
- `reqwest` - HTTP client
- `trust-dns` - DNS client

### WRAITH Core Integration
- `wraith-crypto` - Noise_XX, Double Ratchet, AEAD
- `wraith-obfuscation` - Protocol mimicry, timing
- `wraith-transport` - Multi-transport support
- `wraith-files` - Chunked file transfer
