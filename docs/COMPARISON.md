# WRAITH Protocol Comparison Guide

**Version:** 1.0.0
**Last Updated:** 2025-12-05
**Status:** Complete Protocol Comparison

---

## Table of Contents

1. [Protocol Comparison](#1-protocol-comparison)
2. [Feature Matrix](#2-feature-matrix)
3. [Security Comparison](#3-security-comparison)
4. [Use Case Fit](#4-use-case-fit)

---

## 1. Protocol Comparison

### 1.1 WRAITH vs. QUIC

| Aspect | WRAITH | QUIC |
|--------|--------|------|
| **Transport** | UDP, AF_XDP | UDP |
| **Encryption** | XChaCha20-Poly1305 | AES-GCM, ChaCha20-Poly1305 |
| **Handshake** | Noise_XX (mutual auth) | TLS 1.3 |
| **Forward Secrecy** | Double Ratchet (per-packet + periodic DH) | TLS 1.3 (session-level) |
| **Congestion Control** | BBR | BBR, Cubic, Reno |
| **Connection Migration** | Yes (PATH_CHALLENGE/RESPONSE) | Yes |
| **Stream Multiplexing** | Yes | Yes |
| **Obfuscation** | Elligator2, padding, timing jitter, protocol mimicry | No |
| **NAT Traversal** | STUN, ICE, relay | Relies on application |
| **DHT Discovery** | Built-in Kademlia | No |
| **Zero-Copy I/O** | AF_XDP, io_uring | Platform-dependent |
| **DPI Resistance** | High (Elligator2, mimicry) | Low (identifiable TLS patterns) |
| **Target Use Case** | Privacy-focused P2P file transfer | Web, RPC, streaming |

**When to Choose WRAITH:**
- Privacy is paramount (traffic analysis resistance)
- P2P file sharing without central infrastructure
- Need built-in NAT traversal and peer discovery
- DPI evasion required

**When to Choose QUIC:**
- HTTP/3 web traffic
- RPC between known servers
- Standard TLS certificate infrastructure
- Broad platform support needed

---

### 1.2 WRAITH vs. WireGuard

| Aspect | WRAITH | WireGuard |
|--------|--------|-----------|
| **Use Case** | File transfer, ephemeral sessions | VPN, persistent tunnels |
| **Encryption** | XChaCha20-Poly1305 | ChaCha20-Poly1305 |
| **Key Exchange** | X25519 + Noise_XX | X25519 + Noise_IK |
| **Authentication** | Ed25519 signatures | Pre-shared static keys |
| **Session Model** | Ephemeral (per-transfer) | Persistent tunnel |
| **Kernel Integration** | AF_XDP (userspace) | Kernel module |
| **Ratcheting** | Double Ratchet | Static keys |
| **Post-Compromise Security** | Yes (DH ratchet) | No (must rotate keys manually) |
| **NAT Traversal** | Built-in (STUN, ICE, relay) | Manual keepalives |
| **Peer Discovery** | DHT | Manual configuration |
| **Overhead** | Higher (per-session handshake) | Lower (amortized over tunnel lifetime) |
| **Throughput** | 10-40 Gbps (AF_XDP) | 10-100 Gbps (kernel) |

**When to Choose WRAITH:**
- Ephemeral file transfers
- Dynamic peer discovery needed
- Post-compromise security critical
- P2P without infrastructure

**When to Choose WireGuard:**
- VPN tunnel between known peers
- Long-lived connections
- Maximum performance (kernel bypass)
- Simple configuration

---

### 1.3 WRAITH vs. Noise Protocol (Raw)

| Aspect | WRAITH | Noise Protocol (Raw) |
|--------|--------|----------------------|
| **Scope** | Complete file transfer protocol | Cryptographic framework only |
| **Handshake** | Noise_XX | Customizable (XX, IK, KK, etc.) |
| **Transport** | UDP, AF_XDP with congestion control | BYO transport |
| **Session Management** | Built-in | Not included |
| **Stream Multiplexing** | Built-in | Not included |
| **File Transfer** | Built-in chunking, integrity, resume | Not included |
| **Obfuscation** | Built-in (padding, timing, mimicry) | Not included |
| **DHT/Discovery** | Built-in | Not included |
| **NAT Traversal** | Built-in | Not included |
| **Implementation Complexity** | Complete solution | Framework only |

**When to Choose WRAITH:**
- Need complete file transfer solution
- P2P with discovery and NAT traversal
- Privacy-focused application

**When to Choose Noise (Raw):**
- Building custom protocol
- Need specific handshake pattern
- Integrating into existing transport
- Educational/research purposes

---

### 1.4 WRAITH vs. BitTorrent

| Aspect | WRAITH | BitTorrent |
|--------|--------|------------|
| **Architecture** | Decentralized (DHT) | Centralized trackers + DHT |
| **Encryption** | Always encrypted (XChaCha20-Poly1305) | Optional (RC4 - weak) |
| **Authentication** | Mutual (Noise_XX) | None (unauthenticated) |
| **Piece Verification** | BLAKE3 tree hash | SHA-1 |
| **Discovery** | Privacy-enhanced Kademlia DHT | Mainline DHT |
| **NAT Traversal** | STUN, ICE, relay | UPnP, manual port forwarding |
| **Obfuscation** | Built-in (Elligator2, mimicry) | Limited (protocol encryption) |
| **Resume** | Chunk-level bitmap | Piece selection |
| **Multi-Peer** | Built-in adaptive strategies | Tit-for-tat, rarest-first |
| **DPI Resistance** | High | Low (identifiable handshake) |
| **Throughput** | 300+ Mbps (UDP), 10+ Gbps (AF_XDP) | 100-500 Mbps |

**When to Choose WRAITH:**
- Privacy-focused file sharing
- Authenticated peer-to-peer transfers
- DPI evasion required
- Strong forward secrecy needed

**When to Choose BitTorrent:**
- Public file distribution
- Large swarms (100+ peers)
- Mature client ecosystem
- Web integration (WebTorrent)

---

### 1.5 WRAITH vs. Magic Wormhole

| Aspect | WRAITH | Magic Wormhole |
|--------|--------|----------------|
| **Setup** | DHT discovery or manual peer | PAKE + rendezvous server |
| **Authentication** | Public key (Ed25519) | PAKE code (human-memorable) |
| **Encryption** | XChaCha20-Poly1305 | Spake2 + NaCl secretbox |
| **Discovery** | DHT | Rendezvous server |
| **NAT Traversal** | STUN, ICE, relay | Relay server |
| **Infrastructure** | Optional (can run fully P2P) | Requires rendezvous server |
| **Session Persistence** | Resume support | Single-shot |
| **Multi-File** | Supported | Single file |
| **Scalability** | High (DHT) | Limited (rendezvous server) |
| **User Experience** | Node IDs | Short codes |

**When to Choose WRAITH:**
- No infrastructure dependencies wanted
- Multi-file or repeated transfers
- Advanced obfuscation needed
- High-performance transfers

**When to Choose Magic Wormhole:**
- One-time file transfer
- User-friendly short codes
- Simple setup
- Mutual authentication via PAKE

---

## 2. Feature Matrix

### 2.1 Cryptographic Features

| Feature | WRAITH | QUIC | WireGuard | BitTorrent | Magic Wormhole |
|---------|--------|------|-----------|------------|----------------|
| **AEAD Cipher** | XChaCha20-Poly1305 | AES-GCM, ChaCha20-Poly1305 | ChaCha20-Poly1305 | None (RC4 optional) | NaCl secretbox |
| **Key Exchange** | X25519 | X25519 | X25519 | None | Spake2 |
| **Signatures** | Ed25519 | ECDSA, Ed25519 | None | None | None |
| **Key Hiding** | Elligator2 | No | No | No | No |
| **Forward Secrecy** | Double Ratchet | TLS 1.3 | No | No | Session-level |
| **Post-Compromise Security** | Yes (DH ratchet) | No | No | No | No |
| **Replay Protection** | 64-bit sliding window | TLS 1.3 | Nonce counter | No | Session nonce |
| **Key Commitment** | BLAKE3-based | TLS 1.3 | No | No | No |
| **Hash Function** | BLAKE3 | SHA-256 | BLAKE2s | SHA-1 | SHA-256 |

### 2.2 Transport Features

| Feature | WRAITH | QUIC | WireGuard | BitTorrent | Magic Wormhole |
|---------|--------|------|-----------|------------|----------------|
| **Protocol** | UDP | UDP | UDP | TCP, uTP | TCP |
| **Kernel Bypass** | AF_XDP | No | Kernel module | No | No |
| **Zero-Copy I/O** | io_uring | Platform-dependent | Kernel | No | No |
| **Congestion Control** | BBR | BBR, Cubic | No (relies on IP) | LEDBAT (uTP) | TCP congestion control |
| **Connection Migration** | Yes | Yes | Manual | No | No |
| **Stream Multiplexing** | Yes | Yes | No | No | No |
| **MTU Discovery** | Yes (binary search) | Yes | No | No | No |
| **Packet Pacing** | Yes (BBR-based) | Yes | No | Yes (uTP) | TCP pacing |

### 2.3 Privacy Features

| Feature | WRAITH | QUIC | WireGuard | BitTorrent | Magic Wormhole |
|---------|--------|------|-----------|------------|----------------|
| **Traffic Obfuscation** | Yes (5 modes) | No | No | Limited | No |
| **Timing Obfuscation** | Yes (5 distributions) | No | No | No | No |
| **Protocol Mimicry** | TLS, WebSocket, DoH | No | No | No | No |
| **Cover Traffic** | Yes | No | No | No | No |
| **Key Hiding** | Elligator2 | No | No | No | No |
| **DHT Privacy** | Keyed info_hash | N/A | N/A | None | N/A |
| **DPI Resistance** | High | Low | Low | Low | Medium |

### 2.4 Network Features

| Feature | WRAITH | QUIC | WireGuard | BitTorrent | Magic Wormhole |
|---------|--------|------|-----------|------------|----------------|
| **DHT** | Kademlia | No | No | Mainline DHT | No |
| **NAT Traversal** | STUN, ICE | Application | Keepalives | UPnP | Relay |
| **Relay Fallback** | Yes | Application | No | No | Yes |
| **IPv6** | Yes | Yes | Yes | Yes | Yes |
| **Local Discovery** | mDNS | Application | No | LPD | No |
| **Bootstrap** | Multiple servers | N/A | Static config | Trackers | Rendezvous server |

### 2.5 File Transfer Features

| Feature | WRAITH | QUIC | WireGuard | BitTorrent | Magic Wormhole |
|---------|--------|------|-----------|------------|----------------|
| **Chunking** | Yes (256 KB default) | Application | N/A | Pieces (256 KB - 16 MB) | No |
| **Integrity** | BLAKE3 tree hash | Application | N/A | SHA-1 per piece | SHA-256 |
| **Resume** | Chunk-level bitmap | Application | N/A | Piece selection | No |
| **Multi-Peer** | Yes (4 strategies) | Application | N/A | Yes (tit-for-tat) | No |
| **Compression** | Optional | Application | N/A | No | No |
| **Streaming** | Yes (AV1/VP9/H.264, adaptive bitrate) | Yes | N/A | Planned (WebTorrent) | No |

---

## 3. Security Comparison

### 3.1 Threat Model Comparison

| Threat | WRAITH | QUIC | WireGuard | BitTorrent | Magic Wormhole |
|--------|--------|------|-----------|------------|----------------|
| **Passive Eavesdropping** | ✅ Resistant | ✅ Resistant | ✅ Resistant | ❌ Vulnerable | ✅ Resistant |
| **Active MITM** | ✅ Resistant (mutual auth) | ✅ Resistant (TLS) | ✅ Resistant | ❌ Vulnerable | ✅ Resistant (PAKE) |
| **Traffic Analysis** | ✅ Resistant (obfuscation) | ⚠️ Partial (TLS patterns) | ⚠️ Partial | ❌ Vulnerable | ⚠️ Partial |
| **DPI** | ✅ Resistant (mimicry) | ❌ Vulnerable (TLS fingerprint) | ⚠️ Partial | ❌ Vulnerable | ⚠️ Partial |
| **Replay Attacks** | ✅ Resistant (sliding window) | ✅ Resistant (TLS) | ✅ Resistant (counter) | ❌ No protection | ✅ Resistant (nonce) |
| **Key Compromise** | ✅ Post-compromise security | ⚠️ Forward secrecy only | ❌ Static keys | N/A | ⚠️ Session-level |
| **Sybil Attacks** | ✅ S/Kademlia (20-bit PoW) | N/A | N/A | ⚠️ Partial | N/A |
| **Eclipse Attacks** | ✅ S/Kademlia | N/A | N/A | ⚠️ Partial | N/A |

### 3.2 Security Properties

**WRAITH Security Properties:**
- ✅ Mutual authentication (Ed25519 signatures)
- ✅ Forward secrecy (Double Ratchet)
- ✅ Post-compromise security (DH ratchet every 2 minutes)
- ✅ Replay protection (64-bit sliding window)
- ✅ Traffic analysis resistance (Elligator2, padding, timing jitter)
- ✅ DPI evasion (TLS/WebSocket/DoH mimicry)
- ✅ Key commitment (BLAKE3-based, prevents multi-key attacks)
- ✅ Constant-time operations (all crypto primitives)
- ✅ Memory safety (Rust, ZeroizeOnDrop)

**QUIC Security Properties:**
- ✅ Server authentication (TLS 1.3 certificates)
- ⚠️ Client authentication (optional TLS client certs)
- ✅ Forward secrecy (TLS 1.3)
- ❌ No post-compromise security (must restart handshake)
- ⚠️ Limited traffic analysis resistance
- ❌ No DPI evasion (identifiable TLS patterns)
- ✅ Replay protection (TLS 1.3)

**WireGuard Security Properties:**
- ✅ Mutual authentication (pre-shared static keys)
- ❌ No forward secrecy (static keys)
- ❌ No post-compromise security
- ✅ Replay protection (nonce counter)
- ❌ No traffic analysis resistance
- ❌ No DPI evasion

**BitTorrent Security Properties:**
- ❌ No authentication
- ⚠️ Optional weak encryption (RC4)
- ❌ No forward secrecy
- ❌ No replay protection
- ❌ No traffic analysis resistance
- ❌ No DPI evasion

**Magic Wormhole Security Properties:**
- ✅ Mutual authentication (PAKE)
- ✅ Forward secrecy (Spake2)
- ⚠️ Session-level post-compromise security
- ✅ Replay protection (session nonce)
- ⚠️ Limited traffic analysis resistance
- ❌ No DPI evasion

---

## 4. Use Case Fit

### 4.1 Privacy-Focused File Sharing

**Best Choice:** WRAITH Protocol

**Rationale:**
- End-to-end encryption with mutual authentication
- Traffic obfuscation (Elligator2, padding, timing jitter)
- DPI evasion (TLS/WebSocket/DoH mimicry)
- Post-compromise security (Double Ratchet)
- Decentralized discovery (DHT)
- No metadata leakage

**Alternative:** Magic Wormhole (for simplicity)

---

### 4.2 VPN / Site-to-Site Tunnel

**Best Choice:** WireGuard

**Rationale:**
- Optimized for persistent tunnels
- Kernel-level performance
- Simple configuration
- Battle-tested in production

**Alternative:** QUIC (for HTTP/3 proxying)

---

### 4.3 Web Traffic (HTTP/3)

**Best Choice:** QUIC

**Rationale:**
- Designed for web use cases
- HTTP/3 integration
- TLS 1.3 certificate infrastructure
- Broad browser support

**Alternative:** None (QUIC is standard)

---

### 4.4 Public File Distribution

**Best Choice:** BitTorrent

**Rationale:**
- Mature ecosystem (trackers, DHT, clients)
- Large swarm support (1000+ peers)
- Web integration (WebTorrent)
- CDN-like distribution

**Alternative:** WRAITH (for privacy-enhanced distribution)

---

### 4.5 Ad-Hoc File Transfer

**Best Choice:** Magic Wormhole

**Rationale:**
- User-friendly (short codes)
- Simple setup
- PAKE authentication
- Single-shot transfers

**Alternative:** WRAITH (for repeated transfers or advanced features)

---

### 4.6 High-Performance Data Center

**Best Choice:** WireGuard or QUIC

**Rationale:**
- WireGuard: Kernel-level performance (100+ Gbps)
- QUIC: Application-layer flexibility (10+ Gbps)

**Alternative:** WRAITH with AF_XDP (10-40 Gbps, userspace)

---

### 4.7 Censorship-Resistant Communication

**Best Choice:** WRAITH Protocol

**Rationale:**
- DPI evasion (protocol mimicry)
- Traffic analysis resistance
- Decentralized infrastructure
- No single point of failure

**Alternative:** Tor (for anonymity) + WRAITH (for file transfer)

---

### 4.8 IoT/Embedded Systems

**Best Choice:** WireGuard

**Rationale:**
- Minimal resource requirements
- Small code footprint
- Simple configuration

**Alternative:** QUIC (for HTTP-based IoT)

---

### 4.9 Real-Time Streaming

**Best Choice:** QUIC

**Rationale:**
- Low latency (0-RTT resumption)
- Stream prioritization
- Connection migration
- HTTP/3 integration

**Alternative:** WRAITH (for privacy-enhanced streaming via WRAITH-Stream)

---

### 4.10 Enterprise File Sync

**Best Choice:** BitTorrent Sync or WRAITH

**Rationale:**
- BitTorrent Sync: Mature, feature-rich
- WRAITH: Privacy-focused, decentralized

**Alternative:** Syncthing (open source)

---

## Performance Benchmarks

### Throughput Comparison (10 GbE Network)

| Protocol | Userspace | Kernel Bypass | Notes |
|----------|-----------|---------------|-------|
| WRAITH | 300-500 Mbps | 10-40 Gbps | UDP or AF_XDP |
| QUIC | 1-5 Gbps | N/A | Varies by implementation |
| WireGuard | N/A | 10-100 Gbps | Kernel module |
| BitTorrent | 100-500 Mbps | N/A | TCP or uTP |
| Magic Wormhole | 50-200 Mbps | N/A | TCP |

### Latency Comparison

| Protocol | Handshake (RTT) | Packet Processing |
|----------|-----------------|-------------------|
| WRAITH | 3-RTT (Noise_XX) | <1ms (AF_XDP) |
| QUIC | 1-RTT (or 0-RTT) | ~1-5ms |
| WireGuard | 1-RTT | <1ms (kernel) |
| BitTorrent | N/A (no handshake) | ~5-10ms |
| Magic Wormhole | 2-RTT (PAKE) | ~5-10ms (TCP) |

---

## Conclusion

**WRAITH Protocol** excels in scenarios requiring:
- Privacy and traffic analysis resistance
- DPI evasion capabilities
- Post-compromise security
- Decentralized peer discovery
- Authenticated P2P file transfers

For other use cases, consider:
- **WireGuard:** VPN tunnels, persistent connections
- **QUIC:** HTTP/3 web traffic, RPC
- **BitTorrent:** Public file distribution, large swarms
- **Magic Wormhole:** Simple ad-hoc transfers

Choose the protocol that best fits your threat model, performance requirements, and operational constraints.

---

**WRAITH Protocol Comparison Guide**

**Version:** 1.0.0 | **License:** MIT | **Language:** Rust 2024
