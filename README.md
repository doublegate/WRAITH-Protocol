# WRAITH Protocol

**W**ire-speed **R**esilient **A**uthenticated **I**nvisible **T**ransfer **H**andler

A decentralized secure file transfer protocol optimized for high-throughput, low-latency operation with strong security guarantees and traffic analysis resistance.

<!-- markdownlint-disable no-inline-html -->
<p align="center">
  <img src="images/wraith-protocol_round-patch.jpg" alt="WRAITH Protocol" width="450">
</p>

<p align="center">
  <a href="https://github.com/doublegate/WRAITH-Protocol/actions/workflows/ci.yml"><img src="https://github.com/doublegate/WRAITH-Protocol/actions/workflows/ci.yml/badge.svg" alt="CI Status"></a>
  <a href="https://github.com/doublegate/WRAITH-Protocol/actions/workflows/codeql.yml"><img src="https://github.com/doublegate/WRAITH-Protocol/actions/workflows/codeql.yml/badge.svg" alt="CodeQL"></a>
  <a href="https://github.com/doublegate/WRAITH-Protocol/actions/workflows/release.yml"><img src="https://github.com/doublegate/WRAITH-Protocol/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <br>
  <a href="https://github.com/doublegate/WRAITH-Protocol/releases"><img src="https://img.shields.io/badge/version-2.3.2-blue.svg" alt="Version"></a>
  <a href="docs/security/SECURITY_AUDIT_v1.1.0.md"><img src="https://img.shields.io/badge/security-audited-green.svg" alt="Security"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.88%2B-orange.svg" alt="Rust"></a>
  <a href="https://doc.rust-lang.org/edition-guide/rust-2024/index.html"><img src="https://img.shields.io/badge/edition-2024-orange.svg" alt="Edition"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License"></a>
</p>
<!-- markdownlint-enable no-inline-html -->

---

## Overview

WRAITH Protocol is a privacy-focused, high-performance file transfer protocol designed for secure peer-to-peer communication. Built in Rust, it combines kernel-bypass networking with modern cryptography to deliver:

- **Wire-speed transfers** - 10+ Gbps with AF_XDP kernel bypass, sub-millisecond latency
- **End-to-end encryption** - Noise_XX handshake, XChaCha20-Poly1305, perfect forward secrecy
- **Traffic analysis resistance** - Elligator2 key encoding, protocol mimicry, cover traffic
- **Decentralized discovery** - Kademlia DHT, NAT traversal, relay fallback
- **Production-ready ecosystem** - 12 client applications for file transfer, messaging, and specialized use cases

### Project Metrics

| Metric            | Value                                                                  |
| ----------------- | ---------------------------------------------------------------------- |
| **Tests**         | 2,148 passing (2,123 workspace + 11 spectre-implant + 14 doc), 16 ignored |
| **Code**          | ~141,000 lines Rust (protocol + clients) + ~37,800 lines TypeScript    |
| **Documentation** | 114 files, ~62,800 lines                                               |
| **Security**      | Grade A+ (zero vulnerabilities, 295 audited dependencies)              |
| **Quality**       | 98/100, zero clippy warnings                                           |
| **TDR**           | ~2.5% (Grade A - Excellent)                                            |
| **Applications**  | 12 production-ready clients (9 desktop + 2 mobile + 1 server platform) |
| **Templates**     | 17 configuration/ROE templates                                         |

![WRAITH Protocol Banner](images/wraith-protocol_banner-graphic.jpg)

---

## Features

### High-Performance Transport

- Wire-speed transfers (10+ Gbps with AF_XDP kernel bypass)
- Sub-millisecond latency (<1ms packet processing with io_uring)
- Zero-copy I/O via AF_XDP UMEM and io_uring registered buffers
- BBR congestion control with optimal bandwidth utilization
- Async file I/O with io_uring

### Security and Privacy

- End-to-end encryption (XChaCha20-Poly1305 AEAD)
- Perfect forward secrecy (Double Ratchet with DH and symmetric ratcheting)
- Mutual authentication (Noise_XX handshake pattern)
- Ed25519 digital signatures for identity verification
- BLAKE3 cryptographic hashing
- Traffic analysis resistance (Elligator2 key encoding)
- Replay protection (64-bit sliding window)
- Key commitment for AEAD (prevents multi-key attacks)

### Traffic Obfuscation

- Packet padding (PowerOfTwo, SizeClasses, ConstantRate, Statistical modes)
- Timing obfuscation (Fixed, Uniform, Normal, Exponential distributions)
- Protocol mimicry (TLS 1.3, WebSocket, DNS-over-HTTPS)
- Cover traffic generation (Constant, Poisson, Uniform distributions)
- Adaptive threat-level profiles (Low, Medium, High, Paranoid)

### Decentralized Discovery

- Privacy-enhanced Kademlia DHT with BLAKE3 NodeIds
- S/Kademlia Sybil resistance (crypto puzzle-based NodeId generation)
- NAT traversal (STUN client, ICE-lite UDP hole punching)
- DERP-style relay infrastructure with automatic fallback
- Connection migration with PATH_CHALLENGE/PATH_RESPONSE

### File Transfer

- Chunked file transfer with BLAKE3 tree hashing
- Multi-peer downloads with parallel chunk fetching
- Resume support with missing chunks detection
- Real-time progress tracking (bytes, speed, ETA)
- Chunk verification (<1us per chunk)

![WRAITH Protocol Architecture](images/wraith-protocol_arch-infographic.jpg)

---

## History

WRAITH Protocol draws inspiration from the rich history of network reconnaissance, tracing a lineage from the analog telephone era to modern internet-scale discovery.

### The Wardialing Era (1980s-1990s)

In the formative decades of the digital age, the Public Switched Telephone Network (PSTN) was the primary gateway to computer systems. **Wardialing**—the systematic automated dialing of telephone numbers to identify responding modems—became the foundational reconnaissance technique. The 1980 release of Zoom Telephonics' **Demon Dialer** introduced automated dialing to consumers, while the 1983 film _WarGames_ popularized the technique and directly influenced national security policy, leading to NSDD-145 and the modern information security state.

Tools like **ToneLoc** (1990s) pioneered randomized dialing to evade detection, while **THC-Scan** added European PBX support. The commercial **PhoneSweep** (1998) legitimized wardialing as a professional security audit practice, with banner fingerprinting databases identifying over 470 system types.

### The Digital Evolution (2000s-Present)

As dial-up faded, the wardialing philosophy migrated to new mediums. **Wardriving** applied the same brute-force discovery to Wi-Fi networks. **WarVOX** (2009) leveraged VoIP and audio fingerprinting for cloud-scale telephone scanning. Today, stateless scanners like **Masscan** can enumerate the entire IPv4 address space in minutes, while **Shodan** continuously indexes every internet-connected device—the ultimate evolution of reconnaissance from the 10,000-number telephone exchange to the 4.3 billion-address IPv4 space.

WRAITH Protocol embodies this evolution: the same philosophy of systematic discovery and secure communication, implemented with modern cryptography and kernel-bypass networking for the contemporary threat landscape.

_For the complete history of automated reconnaissance from the Demon Dialer to Shodan, see [The Dial-Up Frontier](ref-docs/Wardialing_Then-Now_History.md)._

![Wardialing History](images/wardialing_then-now_history-graphic.jpg)

---

## Client Applications

WRAITH Protocol powers a comprehensive ecosystem of 12 production-ready applications:

### Tier 1: Core Applications

| Application                                     | Platform | Description                                               |
| ----------------------------------------------- | -------- | --------------------------------------------------------- |
| **[WRAITH-Transfer](clients/wraith-transfer/)** | Desktop  | Secure P2P file transfer with drag-and-drop GUI           |
| **[WRAITH-Chat](clients/wraith-chat/)**         | Desktop  | E2EE messaging with voice/video calls and group messaging |

### Tier 2: Specialized Applications

| Application                                 | Platform | Description                                                              |
| ------------------------------------------- | -------- | ------------------------------------------------------------------------ |
| **[WRAITH-Sync](clients/wraith-sync/)**     | Desktop  | Serverless file synchronization with delta transfers and version history |
| **[WRAITH-Share](clients/wraith-share/)**   | Desktop  | Anonymous distributed file sharing with swarm transfers                  |
| **[WRAITH-Stream](clients/wraith-stream/)** | Desktop  | Secure media streaming with AV1/VP9/H.264 and adaptive bitrate           |

### Tier 3: Advanced & Security Applications

| Application                                   | Platform         | Description                                                                    |
| --------------------------------------------- | ---------------- | ------------------------------------------------------------------------------ |
| **[WRAITH-Mesh](clients/wraith-mesh/)**       | Desktop          | IoT mesh networking with topology visualization and diagnostics                |
| **[WRAITH-Publish](clients/wraith-publish/)** | Desktop          | Censorship-resistant content publishing with Ed25519 signatures and RSS        |
| **[WRAITH-Vault](clients/wraith-vault/)**     | Desktop          | Distributed secret storage with Shamir's Secret Sharing                        |
| **[WRAITH-Recon](clients/wraith-recon/)**     | Desktop          | Network reconnaissance platform for authorized security testing (RoE enforced) |
| **[WRAITH-RedOps](clients/wraith-redops/)**   | Server + Desktop | Red team operations platform with C2 infrastructure and implant framework      |

### Mobile Clients

| Application                                   | Platform     | Description                                                     |
| --------------------------------------------- | ------------ | --------------------------------------------------------------- |
| **[WRAITH-Android](clients/wraith-android/)** | Android 8.0+ | Native Kotlin + Jetpack Compose with JNI bindings to wraith-ffi |
| **[WRAITH-iOS](clients/wraith-ios/)**         | iOS 16.0+    | Native Swift + SwiftUI with UniFFI bindings                     |

### Application Highlights

**WRAITH-Chat** features:

- Signal Protocol Double Ratchet (forward secrecy + post-compromise security)
- Voice calling with Opus codec and RNNoise noise suppression
- Video calling with VP8/VP9 and adaptive bitrate (360p-1080p)
- Group messaging with Sender Keys protocol for O(1) encryption
- SQLCipher encrypted local database

**WRAITH-Vault** features:

- Shamir's Secret Sharing with configurable k-of-n threshold
- Guardian-based key fragment distribution
- Erasure coding (Reed-Solomon) for redundancy
- AES-256-GCM encrypted backups with zstd compression
- Scheduled automatic backups

**WRAITH-Recon** features (authorized security testing only):

- Rules of Engagement (RoE) enforcement with Ed25519 signatures
- Passive reconnaissance with AF_XDP kernel-bypass capture (10-40 Gbps)
- Covert channel testing (DNS tunneling, ICMP steganography, TLS mimicry)
- Tamper-evident audit logging with MITRE ATT&CK mapping
- Kill switch with <1ms activation latency

**WRAITH-RedOps** features (authorized red team operations only):

- Team Server with PostgreSQL persistence, campaign management, gRPC API
- Operator Client (Tauri 2.0) with full-featured UI: 34 IPC commands wired, 21 console commands, zustand state management, toast notifications, context menus, keyboard shortcuts
- Spectre Implant framework (no_std Rust) with C2 loop, sleep obfuscation, API hashing
- Multi-operator support with role-based access control (RBAC)
- Listener management for HTTP, HTTPS, DNS tunneling, TCP, SMB Named Pipe protocols
- Process injection: Reflective DLL, Process Hollowing, Thread Hijack (Windows)
- BOF Loader with COFF parsing, section mapping, and relocation support
- SOCKS4a/5 proxy for tunneling operator traffic
- Halo's Gate SSN resolution for direct syscalls
- MITRE ATT&CK technique coverage across 12 tactics (TA0001-TA0011, TA0040)
- Ed25519-signed Kill Switch broadcast mechanism
- Encryption at Rest for command payloads and results

### WRAITH-RedOps Gap Analysis (v7.0.0)

The RedOps platform has undergone a comprehensive deep audit (v7.0.0) with line-by-line verification of all source files across all three components. The v7.0.0 audit corrected several metrics from v6.0.0, discovering 3 additional implant modules (compression.rs, exfiltration.rs, impact.rs), confirming 100% IPC coverage (32/32 RPCs wired), and expanding MITRE ATT&CK coverage to 87%.

| Metric                           | Value                                                                                    |
| -------------------------------- | ---------------------------------------------------------------------------------------- |
| **Overall Completion**           | ~98% (up from ~97% in v7.0.0)                                                            |
| **Modules**                      | 21 across 3 components                                                                   |
| **MITRE ATT&CK Coverage**        | 87% (35 of 40 techniques implemented)                                                    |
| **P0 Critical Issues**           | 0 (all resolved)                                                                         |
| **P1 High Issues**               | 2 remaining (key ratcheting 13 SP, PowerShell runner 5 SP)                               |
| **Frontend IPC Coverage**        | 34/34 Tauri IPC commands wired (100% -- previously 19/34)                                |
| **Hardcoded Cryptographic Keys** | 0 (all resolved)                                                                         |
| **Story Points Remaining**       | ~59 SP across 13 findings (down from ~73 SP / 17 findings in v6.0.0)                     |
| **Remaining Findings**           | 13 total (0 P0, 2 P1, 5 P2, 6 P3)                                                       |

| Component          | Completion | Delta (from v7.0.0) | Notes                                                                          |
| ------------------ | ---------- | -------------------- | ------------------------------------------------------------------------------ |
| Team Server        | 97%        | +0%                  | 5,833 lines, all 32 RPCs wired, playbook system complete, DNS + SMB listeners  |
| Operator Client    | 99%        | +2%                  | ~5,800 lines, 21 console commands, 34/34 IPC wired, full UI/UX (zustand, toasts, modals, context menus) |
| Spectre Implant    | 95%        | +0%                  | 8,925 lines, 21 modules (+3: compression, exfiltration, impact), 11 tests      |
| WRAITH Integration | 97%        | +0%                  | P2P mesh C2, entropy mixing, SecureBuffer with mlock, PQ crypto integration    |

For the full gap analysis, see [GAP-ANALYSIS-v2.3.0.md](docs/clients/wraith-redops/GAP-ANALYSIS-v2.3.0.md).

For detailed client documentation, see the [Client Overview](docs/clients/overview.md).

---

## Quick Start

### Installation

**Pre-built Binaries** (recommended):

Download from the [releases page](https://github.com/doublegate/WRAITH-Protocol/releases):

| Platform | Architecture    | Notes                   |
| -------- | --------------- | ----------------------- |
| Linux    | x86_64, aarch64 | glibc and musl variants |
| macOS    | x86_64, aarch64 | Intel and Apple Silicon |
| Windows  | x86_64          | -                       |

```bash
# Linux/macOS
tar xzf wraith-<platform>.tar.gz
chmod +x wraith
./wraith --version

# Windows (PowerShell)
Expand-Archive wraith-x86_64-windows.zip
.\wraith.exe --version
```

**Build from Source:**

```bash
git clone https://github.com/doublegate/WRAITH-Protocol.git
cd WRAITH-Protocol

cargo build --release
cargo test --workspace

./target/release/wraith --version
```

**Requirements:**

- Rust 1.88+ (Rust 2024 edition)
- Linux 6.2+ recommended (for AF_XDP and io_uring)
- x86_64 or aarch64 architecture

### Basic Usage

```bash
# Generate identity keypair
wraith keygen --output ~/.wraith/identity.key

# Send a file to peer
wraith send document.pdf <peer-id>

# Receive files
wraith receive --output ./downloads

# Run as background daemon
wraith daemon --bind 0.0.0.0:0

# Check node status
wraith status

# List discovered peers
wraith peers

# View configuration
wraith config --show
```

For detailed usage, see the [User Guide](docs/USER_GUIDE.md) and [Tutorial](docs/TUTORIAL.md).

---

## Architecture

WRAITH Protocol uses a six-layer design optimized for security and performance:

```text
   Application Layer          File transfer, chunking, integrity
         |
   Session Layer              Stream mux, flow control, BBR congestion
         |
   Crypto Transport           Noise_XX, XChaCha20-Poly1305, ratcheting
         |
   Obfuscation Layer          Elligator2, padding, timing jitter
         |
   Kernel Acceleration        AF_XDP, io_uring, zero-copy DMA
         |
   Network Layer              UDP, raw sockets, covert channels
```

### Protocol Crates

| Crate                  | Description                                                  | Tests |
| ---------------------- | ------------------------------------------------------------ | ----- |
| **wraith-core**        | Frame parsing (SIMD), sessions, congestion control, Node API | 456   |
| **wraith-crypto**      | Ed25519, X25519+Elligator2, AEAD, Noise_XX, Double Ratchet   | 216   |
| **wraith-transport**   | AF_XDP, io_uring, UDP sockets, worker pools                  | 183   |
| **wraith-obfuscation** | Padding, timing, cover traffic, protocol mimicry             | 140   |
| **wraith-discovery**   | Kademlia DHT, STUN, ICE, relay infrastructure                | 301   |
| **wraith-files**       | File chunking, BLAKE3 tree hashing, io_uring I/O             | 34    |
| **wraith-cli**         | Command-line interface with Node API integration             | 87    |
| **wraith-ffi**         | Foreign function interface (C/JNI bindings)                  | 111   |

For detailed architecture documentation, see [Protocol Overview](docs/architecture/protocol-overview.md).

---

## Performance

### Targets

| Metric              | Target    | Notes                 |
| ------------------- | --------- | --------------------- |
| Throughput (10 GbE) | >9 Gbps   | AF_XDP with zero-copy |
| Throughput (1 GbE)  | >950 Mbps | With encryption       |
| Handshake Latency   | <50 ms    | LAN conditions        |
| Packet Latency      | <1 ms     | NIC to application    |
| Memory per Session  | <10 MB    | Including buffers     |
| CPU @ 10 Gbps       | <50%      | 8-core system         |

### Benchmarks (v2.3.2)

Measured on production hardware with `cargo bench --workspace`. See [Benchmark Analysis](docs/testing/BENCHMARK-ANALYSIS-v2.3.1.md) for full methodology and results.

| Component            | Measured Performance                        | Details                                    |
| -------------------- | ------------------------------------------- | ------------------------------------------ |
| Frame Parsing        | 2.4 ns/frame (~563 GiB/s equivalent)       | SIMD: AVX2/SSE4.2/NEON, 172M frames/sec   |
| AEAD Encryption      | ~1.4 GiB/s (XChaCha20-Poly1305)            | 256-bit key, 192-bit nonce                 |
| Noise XX Handshake   | 345 us per handshake                        | Full mutual authentication                 |
| Elligator2 Encoding  | 29.5 us per encoding                        | Key indistinguishability from random       |
| BLAKE3 Hashing       | 4.71 GiB/s (tree), 8.5 GB/s (parallel)     | rayon + SIMD acceleration                  |
| File Chunking        | 14.85 GiB/s                                 | io_uring async I/O                         |
| Tree Hashing         | 4.71 GiB/s in-memory, 3.78 GiB/s from disk | Merkle tree with BLAKE3                    |
| Chunk Verification   | 4.78 GiB/s                                  | <1 us per 256 KiB chunk                    |
| File Reassembly      | 5.42 GiB/s                                  | O(m) algorithm, zero-copy                  |
| Ring Buffers (SPSC)  | ~100M ops/sec                               | Cache-line padded, lock-free               |
| Ring Buffers (MPSC)  | ~20M ops/sec                                | CAS-based, 4 producers                     |

---

## Security

### Cryptographic Suite

| Function     | Algorithm          | Security Level                     |
| ------------ | ------------------ | ---------------------------------- |
| Signatures   | Ed25519            | 128-bit                            |
| Key Exchange | X25519             | 128-bit                            |
| Key Encoding | Elligator2         | Traffic analysis resistant         |
| AEAD         | XChaCha20-Poly1305 | 256-bit key, 192-bit nonce         |
| Hash         | BLAKE3             | 128-bit collision resistance       |
| KDF          | HKDF-BLAKE3        | 128-bit                            |
| Handshake    | Noise_XX           | Mutual auth, identity hiding       |
| Ratcheting   | Double Ratchet     | Forward + post-compromise security |

### Security Features

**Cryptographic Guarantees:**

- Forward secrecy via Double Ratchet
- Post-compromise security via DH ratchet
- Replay protection with 64-bit sliding window
- Key commitment prevents multi-key attacks
- Automatic rekey (time, packet-count, byte-count triggers)

**Traffic Analysis Resistance:**

- Elligator2 makes keys indistinguishable from random
- Cover traffic generation (multiple distribution modes)
- Configurable padding modes
- Protocol mimicry (TLS, WebSocket, DoH)

**Implementation Security:**

- Memory-safe Rust with ZeroizeOnDrop for secrets
- Constant-time cryptographic operations
- SIMD acceleration with security validation
- 100% unsafe code documentation

**Validation:**

- Comprehensive test coverage (2,148 tests across all components)
- DPI evasion validation (Wireshark, Zeek, Suricata, nDPI)
- 5 libFuzzer targets
- Property-based tests
- Security scanning (Dependabot, CodeQL, RustSec)

For security issues, see [SECURITY.md](SECURITY.md) for our responsible disclosure process.

---

## Wire Format

WRAITH Protocol uses a compact binary wire format designed for minimal overhead and traffic analysis resistance:

### Outer Packet Structure

```text
+----------+-------------------+----------+
| CID (8B) | Encrypted Payload | Tag (16B)|
+----------+-------------------+----------+
```

- **Connection ID (CID):** 8-byte identifier for session multiplexing
- **Encrypted Payload:** XChaCha20-Poly1305 AEAD-encrypted inner frame
- **Authentication Tag:** 16-byte Poly1305 MAC for integrity verification

### Inner Frame Structure

```text
+----------------+---------+----------------+
| Header (28B)   | Payload | Random Padding |
+----------------+---------+----------------+
```

- **Header Fields:** Frame type (1B), flags (1B), stream ID (4B), sequence number (8B), offset (8B), payload length (4B), reserved (2B)
- **Payload:** Variable-length application data
- **Padding:** Random bytes per configured padding mode (PowerOfTwo, SizeClasses, ConstantRate, Statistical)

### Frame Types

| Type           | Value | Description                       |
| -------------- | ----- | --------------------------------- |
| DATA           | 0x00  | Application data transfer         |
| ACK            | 0x01  | Acknowledgment with selective ACK |
| CONTROL        | 0x02  | Session control signals           |
| REKEY          | 0x03  | Key rotation trigger              |
| PING           | 0x04  | Keepalive probe                   |
| PONG           | 0x05  | Keepalive response                |
| CLOSE          | 0x06  | Graceful session termination      |
| PAD            | 0x07  | Cover traffic padding frame       |
| STREAM_OPEN    | 0x10  | Open new multiplexed stream       |
| STREAM_CLOSE   | 0x11  | Close multiplexed stream          |
| STREAM_REQUEST | 0x12  | Request data chunks               |
| STREAM_DATA    | 0x13  | Deliver data chunks               |
| PATH_CHALLENGE | 0x20  | Connection migration probe        |
| PATH_RESPONSE  | 0x21  | Connection migration confirmation |

---

## Threading Model

WRAITH Protocol employs a thread-per-core architecture designed for maximum throughput with minimal contention:

- **Thread-per-Core:** Each worker thread is pinned to a specific CPU core, eliminating context switches and maximizing cache locality
- **No Locks in Hot Path:** Lock-free ring buffers (SPSC/MPSC) and atomic operations replace mutexes in the data plane
- **Session Pinning:** Each session is assigned to a specific core, ensuring all packet processing for a session stays on the same core
- **NUMA Awareness:** Memory allocation respects NUMA topology on multi-socket systems, minimizing cross-socket memory accesses
- **Batch Processing:** AF_XDP batch receive/transmit operations amortize system call overhead across multiple packets
- **Cache-Line Padding:** Data structures use 64-byte cache-line alignment to prevent false sharing between cores

---

## Development

### Build Commands

```bash
cargo build --workspace           # Development build
cargo build --release             # Release build
cargo test --workspace            # Run all tests
cargo clippy --workspace -- -D warnings  # Linting
cargo fmt --all                   # Format code
cargo xtask ci                    # Full CI suite
cargo doc --workspace --open      # API documentation
cargo bench --workspace           # Benchmarks
```

### Key Dependencies

| Dependency              | Purpose                                                         |
| ----------------------- | --------------------------------------------------------------- |
| `chacha20poly1305`      | XChaCha20-Poly1305 AEAD encryption (256-bit key, 192-bit nonce) |
| `x25519-dalek`          | X25519 Diffie-Hellman key exchange                              |
| `ed25519-dalek`         | Ed25519 digital signatures with batch verification              |
| `curve25519-elligator2` | Elligator2 key encoding for traffic analysis resistance         |
| `blake3`                | BLAKE3 cryptographic hashing with SIMD acceleration             |
| `snow`                  | Noise Protocol Framework (Noise_XX handshake pattern)           |
| `io-uring`              | Linux io_uring async I/O for zero-copy file operations          |
| `tokio`                 | Async runtime for concurrent I/O operations                     |
| `clap`                  | Command-line argument parsing                                   |
| `tauri`                 | Cross-platform desktop application framework (v2.0)             |
| `pnet`                  | Low-level network packet capture and construction               |
| `rusqlite`              | SQLite/SQLCipher encrypted database                             |

### Project Structure

```text
WRAITH-Protocol/
|-- crates/                # Protocol crates (8 active + 1 excluded)
|   |-- wraith-core/       # Frame, session, congestion, Node API
|   |-- wraith-crypto/     # Noise, AEAD, Elligator2, ratcheting
|   |-- wraith-transport/  # AF_XDP, io_uring, UDP
|   |-- wraith-obfuscation/# Padding, timing, mimicry
|   |-- wraith-discovery/  # DHT, relay, NAT traversal
|   |-- wraith-files/      # Chunking, integrity, transfer
|   |-- wraith-cli/        # CLI
|   +-- wraith-ffi/        # FFI bindings
|-- clients/               # Client applications (12)
|   |-- wraith-transfer/   # Desktop file transfer (Tauri 2.0)
|   |-- wraith-chat/       # E2EE messaging (Tauri 2.0)
|   |-- wraith-sync/       # File synchronization (Tauri 2.0)
|   |-- wraith-share/      # Distributed sharing (Tauri 2.0)
|   |-- wraith-stream/     # Media streaming (Tauri 2.0)
|   |-- wraith-mesh/       # IoT mesh networking (Tauri 2.0)
|   |-- wraith-publish/    # Decentralized publishing (Tauri 2.0)
|   |-- wraith-vault/      # Secret storage (Tauri 2.0)
|   |-- wraith-recon/      # Network reconnaissance (Tauri 2.0)
|   |-- wraith-android/    # Android mobile client (Kotlin + JNI)
|   |-- wraith-ios/        # iOS mobile client (Swift + UniFFI)
|   +-- wraith-redops/     # Red team operations platform
|       |-- team-server/       # Axum + gRPC + PostgreSQL (workspace member)
|       |-- operator-client/   # Tauri 2.0 desktop client (workspace member)
|       +-- spectre-implant/   # no_std implant (excluded from workspace)
|-- templates/             # Configuration and ROE templates (17)
|   |-- recon/             # WRAITH-Recon ROE templates (7)
|   |-- config/            # CLI and node configuration (2)
|   |-- transfer/          # Transfer profile templates (1)
|   +-- integration/       # Docker Compose, systemd service (2)
|-- conductor/             # Project management system with code style guides
|-- docs/                  # Documentation (130+ files)
|-- to-dos/                # Project planning
|-- ref-docs/              # Protocol specifications
|-- ref-proj/              # Reference projects (.gitignored, local only)
|-- tests/                 # Integration tests and benchmarks
+-- xtask/                 # Build automation

Workspace: 22 members (8 protocol + 9 Tauri clients + team-server +
           operator-client + xtask + tests)
Excluded:  wraith-xdp (eBPF toolchain), spectre-implant (no_std)
```

---

## CI/CD

WRAITH Protocol uses comprehensive GitHub Actions workflows:

- **Continuous Integration** - Tests, linting, formatting on every push/PR
- **Security Scanning** - Dependabot, CodeQL, cargo-audit, gitleaks, fuzzing
- **Multi-Platform Releases** - 6 platform targets with SHA256 checksums via `cross-rs`
- **Cross-Compilation** - Docker-based builds with `Cross.toml` pre-build hooks (protobuf-compiler)
- **Client Builds** - All Tauri desktop applications (RedOps clients excluded from CI due to frontend asset requirements)

See [CI Workflow](.github/workflows/ci.yml) and [Release Workflow](.github/workflows/release.yml).

---

## Documentation

### Getting Started

- [User Guide](docs/USER_GUIDE.md)
- [Tutorial](docs/TUTORIAL.md)
- [Configuration Reference](docs/CONFIG_REFERENCE.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)

### Architecture Reference

- [Protocol Overview](docs/architecture/protocol-overview.md)
- [Layer Design](docs/architecture/layer-design.md)
- [Security Model](docs/architecture/security-model.md)
- [Performance Architecture](docs/architecture/performance-architecture.md)

### Developer Guide

- [Development Guide](docs/engineering/development-guide.md)
- [Coding Standards](docs/engineering/coding-standards.md)
- [API Reference](docs/engineering/api-reference.md)
- [Integration Guide](docs/INTEGRATION_GUIDE.md)

### Security Resources

- [Security Audit](docs/SECURITY_AUDIT.md)
- [DPI Evasion Report](docs/security/DPI_EVASION_REPORT.md)

### Client Documentation

- [Client Overview](docs/clients/overview.md)
- [UI/UX Design Reference](docs/clients/UI-UX-DESIGN-REFERENCE.md)
- [Client Roadmap](to-dos/ROADMAP-clients.md)

### Templates

- [Templates Overview](templates/README.md)
- [ROE Templates for WRAITH-Recon](templates/recon/README.md)
- [Configuration Templates](templates/config/README.md)
- [Integration Templates](templates/integration/README.md)

### Development History

- [Protocol Development History](docs/archive/README_Protocol-DEV.md)
- [Client Development History](docs/archive/README_Clients-DEV.md)

---

## Roadmap

### Completed

WRAITH Protocol v2.3.2 represents 2,740+ story points across 24 development phases:

- Core protocol implementation (cryptography, transport, obfuscation, discovery)
- 12 production-ready client applications (9 desktop + 2 mobile + 1 server platform)
- WRAITH-RedOps with deep audit gap analysis v7.0.0 (~97% completion, 87% MITRE ATT&CK coverage (35/40), 0 P0 critical issues, ~59 SP remaining across 13 findings)
- RedOps codebase: 8,925 lines spectre-implant, 5,833 lines team-server, ~5,800 lines operator-client (21 modules, 34/34 IPC commands wired, 21 console commands, 11 spectre-implant tests)
- Conductor project management system with code style guides for development workflow tracking
- RedOps workspace integration: team-server and operator-client as workspace members (spectre-implant excluded for no_std compatibility)
- Comprehensive documentation (114 files, ~62,800 lines) and testing (2,134 tests across all components)
- CI/CD infrastructure with multi-platform releases

### Future Development

- **Post-quantum cryptography** - Kyber/Dilithium hybrid mode
- **Formal verification** - Cryptographic protocol proofs
- **XDP/eBPF implementation** - Full kernel bypass (wraith-xdp crate)
- **SDK development** - Python, Go, Node.js language bindings

See [ROADMAP.md](to-dos/ROADMAP.md) for detailed planning.

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick Start to Contribute

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make changes with tests
4. Run CI checks (`cargo xtask ci`)
5. Commit (`git commit -m 'feat: add amazing feature'`)
6. Push and open a Pull Request

### Requirements

- Follow Rust coding standards (rustfmt, clippy)
- Add tests for new functionality
- Update documentation
- Follow [Conventional Commits](https://www.conventionalcommits.org/)

---

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

## Acknowledgments

WRAITH Protocol builds on excellent projects and research:

**Protocol Inspirations:**
[Noise Protocol Framework](https://noiseprotocol.org/) |
[WireGuard](https://www.wireguard.com/) |
[QUIC](https://quicwg.org/) |
[libp2p](https://libp2p.io/) |
[Signal Protocol](https://signal.org/docs/)

**Cryptographic Libraries:**
[RustCrypto](https://github.com/RustCrypto) |
[Snow](https://github.com/mcginty/snow) |
[dalek-cryptography](https://github.com/dalek-cryptography)

**Performance Technologies:**
[AF_XDP](https://www.kernel.org/doc/html/latest/networking/af_xdp.html) |
[io_uring](https://kernel.dk/io_uring.pdf) |
[eBPF/XDP](https://ebpf.io/)

---

## Links

- **Repository:** [github.com/doublegate/WRAITH-Protocol](https://github.com/doublegate/WRAITH-Protocol)
- **Issues:** [GitHub Issues](https://github.com/doublegate/WRAITH-Protocol/issues)
- **Discussions:** [GitHub Discussions](https://github.com/doublegate/WRAITH-Protocol/discussions)
- **Changelog:** [CHANGELOG.md](CHANGELOG.md)
- **Security Policy:** [SECURITY.md](SECURITY.md)

---

**WRAITH Protocol** - _Secure. Fast. Invisible._

**Version:** 2.3.2 | **License:** MIT | **Language:** Rust 2024 (MSRV 1.88) | **Tests:** 2,148 passing (2,123 workspace + 11 spectre-implant + 14 doc) | **Clients:** 12 applications (9 desktop + 2 mobile + 1 server)

**Last Updated:** 2026-01-28
