# WRAITH Protocol Client Applications

**Document Version:** 2.3.0
**Last Updated:** 2026-01-27
**Status:** All 12 Clients Complete (9 Desktop + 2 Mobile + 1 Server)

---

## Overview

WRAITH Protocol provides a suite of client applications built on the core protocol, each optimized for specific use cases. All clients share the same secure, privacy-preserving foundation while offering specialized features for different workflows.

**Common Features:**
- End-to-end encryption (Noise_XX + XChaCha20-Poly1305)
- DHT-based peer discovery
- NAT traversal (UPnP, NAT-PMP, STUN, relay fallback)
- Multi-peer transfers (swarm downloads)
- Forward secrecy and replay protection
- Cross-platform support (Linux, macOS, Windows)

---

## Client Applications

### 1. WRAITH-Transfer

**Purpose:** High-speed file transfer utility

**Use Cases:**
- One-time file transfers
- Large file distribution
- Secure document exchange
- Temporary file sharing

**Key Features:**
- Command-line and GUI interfaces
- Progress tracking and resume support
- Integrity verification (BLAKE3)
- Compression (optional LZ4)
- Multi-source downloads

**Target Users:** General users, system administrators

---

### 2. WRAITH-Chat

**Purpose:** Secure instant messaging

**Use Cases:**
- Private conversations
- Group messaging
- File attachments
- Voice/video calls (Opus codec, VP8/VP9, adaptive bitrate)

**Key Features:**
- End-to-end encrypted messages
- Ephemeral messages (disappearing)
- Group chat support
- File sharing integration
- Desktop notifications

**Target Users:** Privacy-conscious individuals, teams

---

### 3. WRAITH-Sync

**Purpose:** Continuous file synchronization

**Use Cases:**
- Desktop/laptop sync
- Team folder collaboration
- Backup and disaster recovery
- Version control for non-code files

**Key Features:**
- Real-time file watching
- Bi-directional sync
- Conflict resolution
- Selective sync (ignore patterns)
- Bandwidth throttling

**Target Users:** Remote workers, distributed teams

---

### 4. WRAITH-Share

**Purpose:** Peer-to-peer file sharing

**Use Cases:**
- Media distribution
- Software releases
- Public datasets
- Community file libraries

**Key Features:**
- Torrent-like swarm downloads
- Magnet link support
- Seedbox functionality
- Web seed integration
- DHT indexing

**Target Users:** Content creators, open-source projects

---

### 5. WRAITH-Stream

**Purpose:** Live data streaming

**Use Cases:**
- Live video streaming
- Audio broadcasting
- Real-time log streaming
- IoT sensor data

**Key Features:**
- Low-latency streaming
- Adaptive bitrate
- Multi-viewer support
- Recording and replay
- Stream encryption

**Target Users:** Streamers, broadcasters, IoT deployments

---

### 6. WRAITH-Mesh

**Purpose:** Decentralized mesh networking

**Use Cases:**
- Censorship circumvention
- Community networks
- Disaster recovery communications
- Offline-first applications

**Key Features:**
- Multi-hop routing
- Self-healing network
- Bandwidth pooling
- Exit node support
- Mobile mesh (Android/iOS)

**Target Users:** Activists, rural communities, emergency responders

---

### 7. WRAITH-Publish

**Purpose:** Decentralized content publishing

**Use Cases:**
- Blogs and websites
- Software documentation
- Academic papers
- News distribution

**Key Features:**
- Static site hosting
- IPFS integration
- Version control
- Access control (group-based)
- RSS/Atom feeds

**Target Users:** Writers, journalists, academics

---

### 8. WRAITH-Vault

**Purpose:** Secure encrypted storage

**Use Cases:**
- Password manager
- Encrypted file vault
- Secure note-taking
- Key/certificate storage

**Key Features:**
- Client-side encryption
- Multi-device sync
- Emergency access
- Secure sharing
- Audit logging

**Target Users:** Individuals, security teams, enterprises

---

## Security Testing Clients (Tier 3)

**GOVERNANCE NOTICE:** The following clients are specialized security testing tools subject to strict authorization requirements. See [WRAITH Security Testing Parameters](../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md) for complete governance framework.

**Authorized Use Cases Only:**
- Contracted penetration testing engagements
- Executive-authorized red team exercises
- Controlled security research in isolated laboratory environments
- CTF competitions with explicit tool authorization

**Prohibited Uses:** Unauthorized access to any system, indiscriminate scanning, attacks against critical infrastructure, or any use outside documented authorization frameworks.

---

### 9. WRAITH-Recon

**Purpose:** Network reconnaissance and data exfiltration assessment platform for authorized security testing

**Classification:** Tier 3 - Security Testing Tool

**Governance:** Requires signed Rules of Engagement (RoE), scope enforcement, audit logging

**Use Cases (Authorized Only):**
- Network asset discovery without IDS/IPS triggering
- Egress filtering and DLP control validation
- Data exfiltration path mapping
- Network defense capability assessment

**Key Features:**
- **Kernel-Bypass Networking:** AF_XDP for wire-speed reconnaissance (10-40 Gbps, <1ms latency)
- **WRAITH Protocol Integration:** Full cryptographic suite (Noise_XX, XChaCha20-Poly1305, Elligator2)
- **Protocol Mimicry:** TLS 1.3, DNS-over-HTTPS, WebSocket, ICMP traffic patterns
- **Adaptive Obfuscation:** Padding classes (64B-8960B), exponential timing jitter
- **Multi-Path Exfiltration:** Simultaneous data channels via UDP/TCP/HTTPS/DNS/ICMP
- **Passive & Active Scanning:** Zero-touch traffic analysis, stateless connection probing
- **Tamper-Evident Audit Logging:** Cryptographically signed operation logs

**Protocol Stack:**
- **wraith-core:** Session management, stream multiplexing (command/file/SOCKS channels)
- **wraith-crypto:** Noise_XX handshake, BLAKE3 hashing, continuous key ratcheting
- **wraith-transport:** AF_XDP, io_uring, UDP/TCP standard sockets
- **wraith-obfuscation:** Elligator2 encoding, adaptive padding, timing jitter
- **wraith-discovery:** DHT-based relay discovery, multi-hop routing
- **wraith-files:** BLAKE3 chunking, integrity verification, compression

**Safety Controls:**
- Target whitelist enforcement (CIDR/domain restrictions)
- Time-bounded execution (engagement window enforcement)
- Kill switch mechanism (signed halt signal)
- Comprehensive audit trail (all operations logged)

**Target Users:** Authorized penetration testers, red team operators, security researchers

**Documentation:**
- [Architecture](wraith-recon/architecture.md) - Technical deep dive, protocol integration
- [Features](wraith-recon/features.md) - Reconnaissance and exfiltration capabilities
- [Implementation](wraith-recon/implementation.md) - Reference implementation guidance
- [Integration](wraith-recon/integration.md) - Tool compatibility, MITRE ATT&CK mapping
- [Testing](wraith-recon/testing.md) - Protocol verification, evasion effectiveness tests
- [Usage](wraith-recon/usage.md) - Operator workflows, configuration examples

---

### 10. WRAITH-RedOps

**Purpose:** Comprehensive Command & Control (C2) platform for adversary emulation and red team operations

**Classification:** Tier 3 - Security Testing Tool

**Governance:** Requires executive authorization, legal review, incident response coordination

**Use Cases (Authorized Only):**
- Red team exercises simulating advanced threat actors
- Purple team collaborative security assessments
- MITRE ATT&CK technique validation
- EDR/XDR evasion capability testing
- Post-exploitation simulation

**Key Features:**
- **Team Server:** Multi-user collaboration hub (PostgreSQL backend, gRPC API)
- **Operator Console:** Cross-platform GUI (Tauri + React)
- **Spectre Implant:** Memory-resident agent (no_std Rust, zero runtime dependencies)
  - Position Independent Code (PIC) - injectable as shellcode/DLL/EXE
  - Sleep mask obfuscation during idle periods
  - Stack spoofing for call stack legitimacy
  - Indirect syscalls to bypass EDR user-mode hooks
- **WRAITH Protocol C2 Channels:** All transport modes (AF_XDP, UDP, TCP, HTTPS, DNS, ICMP, WebSocket)
- **P2P Beacon Mesh:** Internal lateral movement via SMB pipes, TCP sockets
- **Relay Network:** Multi-hop routing for deep NAT traversal
- **BOF Compatibility:** Run Beacon Object Files from Cobalt Strike ecosystem
- **Multi-Path Data Exfiltration:** Automatic chunking across HTTPS/DNS/ICMP channels

**Protocol Integration:**
- **Noise_XX Handshake:** Mutual authentication, identity hiding (3-phase)
- **XChaCha20-Poly1305 AEAD:** All C2 traffic encrypted (256-bit keys, 192-bit nonces)
- **Elligator2 Encoding:** Handshake keys appear as uniform random bytes
- **Continuous Ratcheting:** Symmetric (per-packet) + DH (2min/1M packets)
- **BBRv2 Congestion Control:** Optimized for C2 performance vs. stealth trade-offs
- **Protocol Mimicry Profiles:** TLS 1.3 (JA3 fingerprinting), DNS-over-HTTPS, WebSocket

**MITRE ATT&CK Coverage:**
- **Initial Access:** Phishing, external remote services
- **Execution:** PowerShell, WMI, scheduled tasks
- **Persistence:** Registry run keys, services, WMI subscriptions
- **Privilege Escalation:** Token manipulation, UAC bypass
- **Defense Evasion:** Obfuscation, process injection, rootkit techniques
- **Credential Access:** LSASS dumping, DCSync, Kerberoasting
- **Discovery:** Network/domain enumeration, user/group discovery
- **Lateral Movement:** SMB/WMI, pass-the-hash/ticket
- **Collection:** Keylogging, screenshot capture, clipboard monitoring
- **Command & Control:** Application layer protocols, encrypted channels
- **Exfiltration:** Over C2 channel, multi-path exfiltration

**Safety Controls:**
- Scope enforcement (target whitelist/blacklist, kernel-side checks)
- Time-to-Live (TTL) - implants self-destruct after engagement end date
- Immutable audit logging (every command, file transfer, lateral movement)
- Emergency kill switch (team server-triggered implant termination)

**Target Users:** Authorized red team operators, purple team assessors, adversary emulation specialists

**Documentation:**
- [Architecture](wraith-redops/architecture.md) - C2 infrastructure, implant design
- [Features](wraith-redops/features.md) - Post-exploitation capabilities
- [Implementation](wraith-redops/implementation.md) - Team server, beacon, operator client
- [Integration](wraith-redops/integration.md) - Protocol stack, external tool compatibility
- [Testing](wraith-redops/testing.md) - Cryptographic verification, evasion testing
- [Usage](wraith-redops/usage.md) - Operator workflows, protocol configuration

---

### Security Testing Parameters Summary

All WRAITH security testing clients are governed by comprehensive authorization and accountability frameworks:

**Authorization Requirements:**
- Written Rules of Engagement (RoE) with explicit scope definition
- Signed authorization from legal system owner
- Liability and indemnification provisions
- Emergency contact procedures

**Technical Safeguards:**
- Cryptographic target whitelisting
- Time-bounded execution windows
- Real-time audit logging with tamper-evident signatures
- Kill switch mechanisms

**Legal Compliance:**
- CFAA (Computer Fraud and Abuse Act) compliance
- Sector-specific regulations (PCI-DSS, HIPAA, GDPR where applicable)
- Coordinated disclosure for discovered vulnerabilities
- Incident response provisions

**For Complete Governance Documentation:** See [WRAITH Security Testing Parameters v1.0](../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)

---

## Architecture Overview

### Shared Components

```
┌─────────────────────────────────────────────────┐
│           Client Application Layer              │
│  (Transfer, Chat, Sync, Share, Stream, etc.)    │
└─────────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────────┐
│         wraith-files (File Operations)          │
└─────────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────────┐
│      wraith-discovery (DHT, Peer Discovery)     │
└─────────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────────┐
│     wraith-transport (AF_XDP, UDP, Relay)       │
└─────────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────────┐
│       wraith-core (Session, Framing, BBR)       │
└─────────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────────┐
│    wraith-crypto (Noise, XChaCha20, BLAKE3)     │
└─────────────────────────────────────────────────┘
```

### Technology Stack

**Backend:**
- Rust (core protocol)
- Tokio (async runtime)
- AF_XDP/io_uring (kernel bypass)

**Frontend:**
- Tauri (desktop GUI)
- React/TypeScript (web UI)
- Flutter (mobile)

**Platform Support:**
- Linux (x86_64, aarch64) - Tier 1
- macOS (x86_64, Apple Silicon) - Tier 2
- Windows (x86_64) - Tier 2
- Android - Tier 2
- iOS - Tier 2

---

## Installation

### Package Managers

**Linux (Debian/Ubuntu):**
```bash
# Add repository
sudo add-apt-repository ppa:wraith/stable
sudo apt update

# Install specific client
sudo apt install wraith-transfer
sudo apt install wraith-chat
sudo apt install wraith-sync
# ... etc
```

**macOS (Homebrew):**
```bash
brew tap wraith/tap
brew install wraith-transfer
brew install wraith-chat
# ... etc
```

**Windows (WinGet):**
```powershell
winget install WRAITH.Transfer
winget install WRAITH.Chat
# ... etc
```

### From Source

```bash
# Clone repository
git clone https://github.com/wraith/wraith-protocol.git
cd wraith-protocol

# Build specific client
cargo build --release -p wraith-transfer
cargo build --release -p wraith-chat
# ... etc

# Install
sudo install target/release/wraith-transfer /usr/local/bin/
```

---

## Quick Start

### WRAITH-Transfer

```bash
# Send file
wraith-transfer send document.pdf --peer 192.0.2.10:41641

# Receive file
wraith-transfer receive --output received.pdf
```

### WRAITH-Chat

```bash
# Start chat client
wraith-chat

# Join group
/join #general <group-secret>

# Send message
Hello, world!
```

### WRAITH-Sync

```bash
# Initialize sync folder
wraith-sync init ~/Documents --group <group-secret>

# Start daemon
wraith-sync daemon

# Check status
wraith-sync status
```

---

## Security Considerations

### Key Management

All clients use the same underlying key management:

1. **Node Keypair:** Ed25519 long-term identity key
2. **Group Secrets:** Symmetric keys for DHT access
3. **Session Keys:** Ephemeral keys (per-session, forward secret)

**Storage:**
- Linux: `~/.config/wraith/keypair.secret`
- macOS: `~/Library/Application Support/wraith/keypair.secret`
- Windows: `%APPDATA%\wraith\keypair.secret`

Permissions: `600` (owner read/write only)

### Privacy

**Metadata Protection:**
- Packet size uniformity (padding)
- Timing obfuscation (cover traffic)
- DHT query unlinkability
- No centralized logging

**Traffic Analysis Resistance:**
- Always-encrypted (no plaintext metadata)
- Relay-blind forwarding
- Obfuscation layer (optional)

---

## Performance Comparison

| Client | Throughput | Latency | Memory | CPU |
|--------|-----------|---------|--------|-----|
| Transfer | 10 Gbps | <5 ms | 50 MB | 25% |
| Chat | N/A | <10 ms | 30 MB | 5% |
| Sync | 5 Gbps | <100 ms | 100 MB | 15% |
| Share | 8 Gbps | <20 ms | 200 MB | 30% |
| Stream | 100 Mbps | <50 ms | 150 MB | 40% |
| Mesh | 1 Gbps | <200 ms | 80 MB | 20% |
| Publish | 2 Gbps | <50 ms | 60 MB | 10% |
| Vault | N/A | <10 ms | 40 MB | 5% |

*Values are typical for standard hardware. AF_XDP can achieve 2-3x higher throughput.*

---

## Interoperability

### Protocol Compatibility

All WRAITH clients are fully interoperable:
- Chat client can send files using Transfer protocol
- Sync can use Share for swarm downloads
- Vault can stream large files via Stream

### Third-Party Integration

**APIs available:**
- REST API (HTTP gateway)
- gRPC API (for services)
- WebSocket API (for web apps)
- FFI bindings (C/C++, Python)

---

## Client-Specific Documentation

For detailed information about each client, see:

- **WRAITH-Transfer:** [architecture](wraith-transfer/architecture.md) | [features](wraith-transfer/features.md) | [implementation](wraith-transfer/implementation.md)
- **WRAITH-Chat:** [architecture](wraith-chat/architecture.md) | [features](wraith-chat/features.md) | [implementation](wraith-chat/implementation.md)
- **WRAITH-Sync:** [architecture](wraith-sync/architecture.md) | [features](wraith-sync/features.md) | [implementation](wraith-sync/implementation.md)
- **WRAITH-Share:** [architecture](wraith-share/architecture.md) | [features](wraith-share/features.md) | [implementation](wraith-share/implementation.md)
- **WRAITH-Stream:** [architecture](wraith-stream/architecture.md) | [features](wraith-stream/features.md) | [implementation](wraith-stream/implementation.md)
- **WRAITH-Mesh:** [architecture](wraith-mesh/architecture.md) | [features](wraith-mesh/features.md) | [implementation](wraith-mesh/implementation.md)
- **WRAITH-Publish:** [architecture](wraith-publish/architecture.md) | [features](wraith-publish/features.md) | [implementation](wraith-publish/implementation.md)
- **WRAITH-Vault:** [architecture](wraith-vault/architecture.md) | [features](wraith-vault/features.md) | [implementation](wraith-vault/implementation.md)

**Mobile Clients:**
- **WRAITH-Android:** Kotlin + JNI, Android Keystore, FCM push notifications
- **WRAITH-iOS:** Swift + UniFFI, iOS Keychain, APNs push notifications

**Security Testing Clients (Tier 3 - Authorized Use Only):**
- **WRAITH-Recon:** [architecture](wraith-recon/architecture.md) | [features](wraith-recon/features.md) | [implementation](wraith-recon/implementation.md) | [integration](wraith-recon/integration.md) | [testing](wraith-recon/testing.md) | [usage](wraith-recon/usage.md)
- **WRAITH-RedOps:** [architecture](wraith-redops/architecture.md) | [features](wraith-redops/features.md) | [implementation](wraith-redops/implementation.md) | [integration](wraith-redops/integration.md) | [testing](wraith-redops/testing.md) | [usage](wraith-redops/usage.md)

---

## Roadmap

**Completed (as of v2.3.0):**
- All 12 client applications implemented and tested
- 9 desktop clients (Transfer, Chat, Sync, Share, Stream, Mesh, Publish, Vault, Recon)
- 2 mobile clients (Android, iOS)
- 1 server platform (RedOps Team Server + Operator Client + Spectre Implant)
- 2,120+ tests passing across all components

**Future Enhancements:**
- Browser extension
- Hardware encryption support (Secure Enclave, TPM)
- Enterprise features (LDAP, SSO)
- Post-quantum cryptography hybrid mode (Kyber/Dilithium)

---

## Community

- **GitHub:** https://github.com/wraith/wraith-protocol
- **Discord:** https://discord.gg/wraith
- **Forum:** https://forum.wraith.network
- **Twitter:** @WraithProtocol

---

## See Also

- [Protocol Overview](../architecture/protocol-overview.md)
- [Security Model](../architecture/security-model.md)
- [Embedding Guide](../integration/embedding-guide.md)
