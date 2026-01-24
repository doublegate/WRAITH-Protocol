# WRAITH Protocol v2 Architecture Overview

**Document Version:** 2.0.0  
**Status:** Architecture Reference  
**Date:** January 2026  

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [System Architecture](#2-system-architecture)
3. [Component Architecture](#3-component-architecture)
4. [Security Architecture](#4-security-architecture)
5. [Network Architecture](#5-network-architecture)
6. [Platform Architecture](#6-platform-architecture)
7. [Deployment Architectures](#7-deployment-architectures)
8. [Data Flow Architecture](#8-data-flow-architecture)
9. [Extension Architecture](#9-extension-architecture)
10. [Performance Architecture](#10-performance-architecture)

---

## 1. Introduction

### 1.1 Purpose

This document describes the overall architecture of the WRAITH Protocol v2 implementation. It provides a comprehensive view of system components, their interactions, and the design decisions that shape the protocol's behavior.

### 1.2 Scope

The architecture covers:
- Core protocol implementation
- Transport abstraction layer
- Cryptographic subsystem
- Obfuscation engine
- Session management
- NAT traversal and discovery
- Platform-specific optimizations

### 1.3 Design Goals

| Goal | Description | Priority |
|------|-------------|----------|
| Security | Post-quantum hybrid cryptography, forward secrecy, authentication | Critical |
| Privacy | Traffic analysis resistance, metadata protection | Critical |
| Performance | High throughput, low latency, kernel bypass capability | High |
| Flexibility | Multiple transports, platforms, use cases | High |
| Simplicity | Clean abstractions, maintainable codebase | Medium |
| Extensibility | Support for future enhancements | Medium |

---

## 2. System Architecture

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              WRAITH Protocol v2                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        Application Interface                         │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐  │   │
│  │  │    File     │  │   Stream    │  │   Message   │  │   Group    │  │   │
│  │  │  Transfer   │  │     API     │  │     API     │  │    API     │  │   │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └─────┬──────┘  │   │
│  └─────────┼────────────────┼────────────────┼───────────────┼─────────┘   │
│            │                │                │               │             │
│  ┌─────────▼────────────────▼────────────────▼───────────────▼─────────┐   │
│  │                        Protocol Core                                 │   │
│  │  ┌───────────────────────────────────────────────────────────────┐  │   │
│  │  │                     Session Manager                            │  │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐    │  │   │
│  │  │  │   Stream    │  │    Flow     │  │     Congestion      │    │  │   │
│  │  │  │ Multiplexer │  │   Control   │  │      Control        │    │  │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────────────┘    │  │   │
│  │  └───────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────┬───────────────────────────────────┘   │
│                                    │                                       │
│  ┌─────────────────────────────────▼───────────────────────────────────┐   │
│  │                        Security Layer                                │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐  │   │
│  │  │   Crypto    │  │  Handshake  │  │  Ratcheting │  │    Key     │  │   │
│  │  │   Engine    │  │   (Noise)   │  │   Engine    │  │   Store    │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘  │   │
│  └─────────────────────────────────┬───────────────────────────────────┘   │
│                                    │                                       │
│  ┌─────────────────────────────────▼───────────────────────────────────┐   │
│  │                       Obfuscation Layer                              │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐  │   │
│  │  │  Elligator  │  │   Padding   │  │   Timing    │  │   Probing  │  │   │
│  │  │   Encoder   │  │   Engine    │  │ Obfuscator  │  │ Resistance │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘  │   │
│  └─────────────────────────────────┬───────────────────────────────────┘   │
│                                    │                                       │
│  ┌─────────────────────────────────▼───────────────────────────────────┐   │
│  │                    Transport Abstraction Layer                       │   │
│  │  ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐       │   │
│  │  │  UDP  │ │  TCP  │ │  WS   │ │ HTTP2 │ │ QUIC  │ │ AF_XDP│       │   │
│  │  └───────┘ └───────┘ └───────┘ └───────┘ └───────┘ └───────┘       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Layer Responsibilities

| Layer | Responsibility | Key Components |
|-------|----------------|----------------|
| Application Interface | User-facing APIs | File, Stream, Message, Group APIs |
| Protocol Core | Session and stream management | Multiplexer, Flow/Congestion Control |
| Security Layer | Cryptographic operations | Noise, AEAD, Ratcheting, Key Management |
| Obfuscation Layer | Traffic analysis resistance | Elligator2, Padding, Timing, Probing |
| Transport Layer | Network I/O abstraction | UDP, TCP, WebSocket, HTTP/2, QUIC, AF_XDP |

### 2.3 Cross-Cutting Concerns

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Cross-Cutting Services                             │
├───────────────┬───────────────┬───────────────┬───────────────┬─────────────┤
│   Logging &   │    Error      │   Resource    │  Configuration │   Metrics   │
│   Tracing     │   Handling    │  Management   │   Management   │  Collection │
├───────────────┼───────────────┼───────────────┼───────────────┼─────────────┤
│ • Structured  │ • Error codes │ • Memory pools│ • Profiles    │ • Bandwidth │
│   logging     │ • Recovery    │ • Buffer mgmt │ • Runtime cfg │ • Latency   │
│ • Span traces │ • Graceful    │ • Thread pools│ • Feature     │ • Packet    │
│ • Debug hooks │   degradation │ • I/O handles │   flags       │   counts    │
└───────────────┴───────────────┴───────────────┴───────────────┴─────────────┘
```

---

## 3. Component Architecture

### 3.1 Session Manager

The Session Manager is the central coordinator for all protocol operations.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Session Manager                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        Session State Machine                         │   │
│  │                                                                       │   │
│  │    CLOSED ──► CONNECTING ──► ESTABLISHED ──► DRAINING ──► CLOSED    │   │
│  │                    │              │              ▲                    │   │
│  │                    │              ├──► REKEYING ─┘                    │   │
│  │                    │              └──► MIGRATING ─┘                   │   │
│  │                    │              └──► RESUMING ──┘                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────────┐  │
│  │   Session Pool   │  │   Stream Pool    │  │   Connection ID Router   │  │
│  │                  │  │                  │  │                          │  │
│  │  • Active        │  │  • Open streams  │  │  • CID → Session map    │  │
│  │    sessions      │  │  • Stream states │  │  • CID rotation         │  │
│  │  • Pending       │  │  • Priorities    │  │  • Collision handling   │  │
│  │    connections   │  │  • Metadata      │  │                          │  │
│  └──────────────────┘  └──────────────────┘  └──────────────────────────┘  │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                         Event Loop                                    │  │
│  │                                                                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │  │
│  │  │   Packet    │  │   Timer     │  │    User     │  │  Internal   │  │  │
│  │  │   Events    │  │   Events    │  │   Events    │  │   Events    │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Crypto Engine

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Crypto Engine                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                         Key Management                                 │ │
│  │                                                                         │ │
│  │  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐    │ │
│  │  │  Static Keys    │    │ Ephemeral Keys  │    │   Group Keys    │    │ │
│  │  │                 │    │                 │    │                 │    │ │
│  │  │ • Identity      │    │ • Per-session   │    │ • TreeKEM       │    │ │
│  │  │ • Long-term     │    │ • Per-ratchet   │    │ • Group secret  │    │ │
│  │  │ • Ed25519       │    │ • X25519+MLKEM  │    │ • Member keys   │    │ │
│  │  └─────────────────┘    └─────────────────┘    └─────────────────┘    │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                       Cryptographic Operations                         │ │
│  │                                                                         │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │ │
│  │  │    AEAD     │  │     KEM     │  │     KDF     │  │   Signing   │   │ │
│  │  │             │  │             │  │             │  │             │   │ │
│  │  │ XChaCha20-  │  │ X25519 +    │  │ HKDF-BLAKE3 │  │  Ed25519    │   │ │
│  │  │ Poly1305    │  │ ML-KEM-768  │  │             │  │             │   │ │
│  │  │ AES-256-GCM │  │             │  │             │  │             │   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                         Ratcheting Engine                              │ │
│  │                                                                         │ │
│  │  ┌─────────────────────────┐    ┌─────────────────────────────────┐   │ │
│  │  │    Symmetric Ratchet    │    │        DH Ratchet               │   │ │
│  │  │                         │    │                                  │   │ │
│  │  │  chain_key[n+1] =       │    │  Trigger: time OR packet count  │   │ │
│  │  │    BLAKE3(chain_key[n]  │    │  new_dh = DH(new_eph, peer_eph) │   │ │
│  │  │           || 0x01)      │    │  chain' = HKDF(chain || new_dh) │   │ │
│  │  │                         │    │                                  │   │ │
│  │  │  Per-packet key deriv   │    │  Forward + post-compromise sec  │   │ │
│  │  └─────────────────────────┘    └─────────────────────────────────┘   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 Obfuscation Engine

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Obfuscation Engine                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Wire Format Polymorphism                          │   │
│  │                                                                       │   │
│  │  Session Secret ───► Format Derivation ───► Unique Wire Layout      │   │
│  │                                                                       │   │
│  │  • Field order randomization                                          │   │
│  │  • Field size variation                                               │   │
│  │  • Dummy field insertion                                              │   │
│  │  • Protocol mimicry (TLS, HTTP, etc.)                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌──────────────────────────┐  ┌──────────────────────────────────────┐   │
│  │      Elligator2          │  │         Padding Engine               │   │
│  │                          │  │                                      │   │
│  │  • Key encoding          │  │  • Continuous distributions         │   │
│  │  • ~50% encodable        │  │  • HTTPS empirical matching         │   │
│  │  • Uniform random output │  │  • Application profiles             │   │
│  │  • High bit randomization│  │  • Adaptive learning                │   │
│  └──────────────────────────┘  └──────────────────────────────────────┘   │
│                                                                             │
│  ┌──────────────────────────┐  ┌──────────────────────────────────────┐   │
│  │    Timing Obfuscator     │  │       Probing Resistance             │   │
│  │                          │  │                                      │   │
│  │  • Distribution matching │  │  • Proof-of-knowledge               │   │
│  │  • HMM timing models     │  │  • Protocol mimicry responses       │   │
│  │  • Cover traffic         │  │  • Service fronting                 │   │
│  │  • Decoy streams         │  │  • Real backend proxy               │   │
│  └──────────────────────────┘  └──────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Entropy Normalization                            │   │
│  │                                                                       │   │
│  │  Raw Ciphertext (~8 bits/byte) ───► Normalized Output (~7 bits/byte) │   │
│  │                                                                       │   │
│  │  Methods: Predictable insertion, Base64, JSON wrapper, HTTP chunked  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.4 Transport Abstraction

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Transport Abstraction Layer                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       Transport Interface                            │   │
│  │                                                                       │   │
│  │  trait Transport {                                                    │   │
│  │      async fn send(&self, packet: &[u8]) -> Result<()>;              │   │
│  │      async fn recv(&self, buf: &mut [u8]) -> Result<usize>;          │   │
│  │      fn characteristics(&self) -> TransportCharacteristics;           │   │
│  │      fn mtu(&self) -> usize;                                          │   │
│  │  }                                                                    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Transport Implementations                         │   │
│  │                                                                       │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐ │   │
│  │  │   UDP   │ │   TCP   │ │WebSocket│ │ HTTP/2  │ │     QUIC        │ │   │
│  │  │         │ │         │ │         │ │         │ │                 │ │   │
│  │  │Unreliable│ │Reliable │ │Reliable │ │Reliable │ │Reliable/Unreli. │ │   │
│  │  │Unordered│ │ Ordered │ │ Ordered │ │ Ordered │ │    Streams      │ │   │
│  │  │  Fast   │ │ Common  │ │Firewall │ │Firewall │ │    Modern       │ │   │
│  │  │         │ │         │ │ friendly│ │ friendly│ │                 │ │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────────────┘ │   │
│  │                                                                       │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────────────────┐ │   │
│  │  │  ICMP   │ │   DNS   │ │Raw Ether│ │        AF_XDP              │ │   │
│  │  │         │ │         │ │         │ │                             │ │   │
│  │  │ Covert  │ │ Covert  │ │ Bypass  │ │   Zero-copy kernel bypass  │ │   │
│  │  │Low B/W  │ │ Tunnel  │ │ Custom  │ │   10-100 Gbps capable      │ │   │
│  │  │         │ │         │ │ Headers │ │   Linux 5.x+ only          │ │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Transport Selection Logic                         │   │
│  │                                                                       │   │
│  │  Requirements → Capability Check → Availability Test → Selection    │   │
│  │                                                                       │   │
│  │  Priority (Stealth):     WebSocket > HTTP/2 > TCP > QUIC > UDP      │   │
│  │  Priority (Performance): AF_XDP > UDP > QUIC > TCP > WebSocket      │   │
│  │  Priority (Balanced):    QUIC > UDP > WebSocket > TCP > HTTP/2      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.5 Congestion and Flow Control

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Congestion and Flow Control                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     BBRv2 Congestion Control                         │   │
│  │                                                                       │   │
│  │           ┌──────────┐                                               │   │
│  │           │ STARTUP  │ ◄─── Exponential bandwidth probing            │   │
│  │           └────┬─────┘                                               │   │
│  │                │ bandwidth plateau                                    │   │
│  │                ▼                                                      │   │
│  │           ┌──────────┐                                               │   │
│  │           │  DRAIN   │ ◄─── Reduce queue                             │   │
│  │           └────┬─────┘                                               │   │
│  │                │ in-flight ≤ BDP                                     │   │
│  │                ▼                                                      │   │
│  │           ┌──────────┐                                               │   │
│  │  ┌───────►│ PROBE_BW │◄──────┐ ◄─── Steady state (8 phases)         │   │
│  │  │        └────┬─────┘       │                                       │   │
│  │  │             │ every 10s   │                                       │   │
│  │  │             ▼             │                                       │   │
│  │  │        ┌──────────┐       │                                       │   │
│  │  └────────│PROBE_RTT │───────┘ ◄─── Measure minimum RTT             │   │
│  │           └──────────┘                                               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌────────────────────────────┐  ┌─────────────────────────────────────┐   │
│  │       Flow Control         │  │           Pacer                      │   │
│  │                            │  │                                      │   │
│  │  Connection Window         │  │  Token bucket algorithm             │   │
│  │  ├── Max: 16 MiB          │  │  ├── Rate from BBR                  │   │
│  │  └── Per-stream windows   │  │  ├── Burst limit (10ms worth)       │   │
│  │                            │  │  └── Timing queue                   │   │
│  │  WINDOW_UPDATE frames      │  │                                      │   │
│  │  ├── Stream-level         │  │  Integration with timing            │   │
│  │  └── Connection-level     │  │  obfuscation layer                  │   │
│  └────────────────────────────┘  └─────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       Loss Detection                                 │   │
│  │                                                                       │   │
│  │  Time-based:   packet_lost if elapsed > 1.5 × SRTT                  │   │
│  │  Packet-based: packet_lost if 3+ later packets ACKed                │   │
│  │                                                                       │   │
│  │  PTO = SRTT + max(4 × RTT_VAR, 1ms) + max_ack_delay                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Security Architecture

### 4.1 Trust Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Trust Model                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Trust Boundaries                             │   │
│  │                                                                       │   │
│  │  ┌───────────────────────────────────────────────────────────────┐   │   │
│  │  │                    Trusted (Endpoints)                         │   │   │
│  │  │                                                                 │   │   │
│  │  │    Local Device          ◄───────────────►      Peer Device    │   │   │
│  │  │    ┌─────────────┐                           ┌─────────────┐   │   │   │
│  │  │    │ Application │                           │ Application │   │   │   │
│  │  │    │   WRAITH    │                           │   WRAITH    │   │   │   │
│  │  │    │   Library   │                           │   Library   │   │   │   │
│  │  │    └─────────────┘                           └─────────────┘   │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  │                              │                                       │   │
│  │  ┌───────────────────────────▼───────────────────────────────────┐   │   │
│  │  │                   Untrusted (Network)                          │   │   │
│  │  │                                                                 │   │   │
│  │  │   ISPs, Routers, Firewalls, Nation-State Observers            │   │   │
│  │  │   Active Attackers, Malicious Relays, DHT Nodes               │   │   │
│  │  │                                                                 │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      Security Assumptions                            │   │
│  │                                                                       │   │
│  │  ✓ Endpoints are not compromised                                     │   │
│  │  ✓ Cryptographic primitives are secure (classical + PQ)             │   │
│  │  ✓ Random number generation is unpredictable                        │   │
│  │  ✓ Timing side-channels are mitigated (not eliminated)              │   │
│  │                                                                       │   │
│  │  ✗ Network is actively hostile                                       │   │
│  │  ✗ Traffic may be recorded for future cryptanalysis                 │   │
│  │  ✗ Adversary has significant computational resources                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Key Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Key Hierarchy                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                          ┌────────────────────┐                            │
│                          │    Identity Key    │                            │
│                          │    (Ed25519)       │                            │
│                          │    Long-term       │                            │
│                          └─────────┬──────────┘                            │
│                                    │                                       │
│              ┌─────────────────────┼─────────────────────┐                 │
│              │                     │                     │                 │
│              ▼                     ▼                     ▼                 │
│    ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐       │
│    │   Static DH Key  │  │  Static PQ Key   │  │  Signing Key     │       │
│    │   (X25519)       │  │  (ML-KEM-768)    │  │  (Ed25519)       │       │
│    │   Per-device     │  │  Per-device      │  │  Same as identity│       │
│    └────────┬─────────┘  └────────┬─────────┘  └──────────────────┘       │
│             │                     │                                        │
│             └──────────┬──────────┘                                        │
│                        │                                                   │
│                        ▼                                                   │
│              ┌──────────────────┐                                          │
│              │  Session Secret  │ ◄─── Noise_XX handshake output          │
│              │  (128-160 bytes) │                                          │
│              └────────┬─────────┘                                          │
│                       │                                                    │
│         ┌─────────────┼─────────────┬─────────────┐                       │
│         │             │             │             │                        │
│         ▼             ▼             ▼             ▼                        │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐              │
│  │ Send Key   │ │ Recv Key   │ │ Wire Format│ │ Padding    │              │
│  │ (32 bytes) │ │ (32 bytes) │ │ Seed       │ │ Seed       │              │
│  └─────┬──────┘ └────────────┘ └────────────┘ └────────────┘              │
│        │                                                                   │
│        ▼                                                                   │
│  ┌────────────┐     ┌────────────┐                                        │
│  │Chain Key[0]│────►│Chain Key[1]│────► ... (symmetric ratchet)           │
│  └─────┬──────┘     └─────┬──────┘                                        │
│        │                  │                                                │
│        ▼                  ▼                                                │
│  ┌────────────┐     ┌────────────┐                                        │
│  │Msg Key[0]  │     │Msg Key[1]  │     ... (per-packet keys)              │
│  └────────────┘     └────────────┘                                        │
│                                                                             │
│  DH Ratchet (every 2 min):                                                 │
│  ┌────────────┐                                                            │
│  │New Ephemeral│───► New DH Secret ───► New Chain Key                     │
│  └────────────┘                                                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.3 Threat Mitigations

| Threat | Mitigation | Component |
|--------|------------|-----------|
| Passive eavesdropping | AEAD encryption | Crypto Engine |
| Man-in-the-middle | Noise_XX mutual authentication | Handshake |
| Replay attacks | Nonce + sliding window | Session Manager |
| Traffic analysis (size) | Continuous padding distribution | Obfuscation |
| Traffic analysis (timing) | Timing obfuscation + cover traffic | Obfuscation |
| Protocol fingerprinting | Wire format polymorphism | Obfuscation |
| Active probing | Proof-of-knowledge + mimicry | Probing Resistance |
| Quantum cryptanalysis | Hybrid X25519 + ML-KEM-768 | Crypto Engine |
| Key compromise | Forward secrecy (per-packet + DH ratchet) | Ratcheting |
| Post-compromise | DH ratchet every 2 minutes | Ratcheting |

---

## 5. Network Architecture

### 5.1 Connection Establishment

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Connection Establishment                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│    Initiator                                              Responder         │
│        │                                                      │             │
│        │  1. Discover responder endpoints (DHT/relay)         │             │
│        │─────────────────────────────────────────────────────►│             │
│        │                                                      │             │
│        │  2. Select transport (UDP/TCP/WS/etc.)              │             │
│        │─────────────────────────────────────────────────────►│             │
│        │                                                      │             │
│        │  3. Phase 1: Proof + Ephemeral (+ PQ key)           │             │
│        │─────────────────────────────────────────────────────►│             │
│        │                                                      │             │
│        │                    (verify proof)                    │             │
│        │                                                      │             │
│        │  4. Phase 2: Ephemeral + Encrypted Static            │             │
│        │◄─────────────────────────────────────────────────────│             │
│        │                                                      │             │
│        │  5. Phase 3: Encrypted Static + Extensions           │             │
│        │─────────────────────────────────────────────────────►│             │
│        │                                                      │             │
│        │  ═══════════ Session Established ═══════════════════ │             │
│        │                                                      │             │
│        │  6. STREAM_OPEN (start transfer)                     │             │
│        │─────────────────────────────────────────────────────►│             │
│        │                                                      │             │
│        │  7. DATA frames                                      │             │
│        │◄────────────────────────────────────────────────────►│             │
│        │                                                      │             │
│                                                                             │
│    Total handshake: 1.5 RTT (3 messages)                                   │
│    First data: 2 RTT from connection start                                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 NAT Traversal Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NAT Traversal Architecture                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                         ┌─────────────────────┐                            │
│                         │    Relay Network    │                            │
│                         │   (DERP-style)      │                            │
│                         └──────────┬──────────┘                            │
│                                    │                                       │
│           ┌────────────────────────┼────────────────────────┐              │
│           │                        │                        │              │
│           ▼                        ▼                        ▼              │
│    ┌─────────────┐          ┌─────────────┐          ┌─────────────┐      │
│    │   Relay 1   │          │   Relay 2   │          │   Relay 3   │      │
│    │  (Region A) │          │  (Region B) │          │  (Region C) │      │
│    └──────┬──────┘          └──────┬──────┘          └──────┬──────┘      │
│           │                        │                        │              │
│           └────────────────────────┼────────────────────────┘              │
│                                    │                                       │
│                      ┌─────────────┴─────────────┐                         │
│                      │                           │                         │
│              ┌───────▼───────┐           ┌───────▼───────┐                 │
│              │     NAT A     │           │     NAT B     │                 │
│              │  (Symmetric)  │           │ (Port-Restr.) │                 │
│              └───────┬───────┘           └───────┬───────┘                 │
│                      │                           │                         │
│              ┌───────▼───────┐           ┌───────▼───────┐                 │
│              │    Peer A     │           │    Peer B     │                 │
│              └───────────────┘           └───────────────┘                 │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      NAT Traversal Flow                              │   │
│  │                                                                       │   │
│  │  1. Endpoint Discovery                                                │   │
│  │     ├── Local addresses                                               │   │
│  │     ├── STUN-like queries to relays                                  │   │
│  │     └── Determine NAT type                                           │   │
│  │                                                                       │   │
│  │  2. Endpoint Exchange (via relay signaling)                          │   │
│  │     ├── Share discovered endpoints                                    │   │
│  │     └── Share NAT type information                                   │   │
│  │                                                                       │   │
│  │  3. Hole Punching Attempt                                            │   │
│  │     ├── Standard: simultaneous UDP probes                            │   │
│  │     ├── Birthday attack: for symmetric NAT                           │   │
│  │     └── TCP simultaneous open (if UDP blocked)                       │   │
│  │                                                                       │   │
│  │  4. Path Validation                                                   │   │
│  │     ├── PATH_CHALLENGE / PATH_RESPONSE                               │   │
│  │     └── Migrate from relay to direct path                            │   │
│  │                                                                       │   │
│  │  5. Fallback to Relay                                                 │   │
│  │     └── If direct connection fails, use relay permanently            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.3 Discovery Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Discovery Architecture                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Privacy-Enhanced DHT                             │   │
│  │                                                                       │   │
│  │  Standard DHT: key = hash(content) ← Anyone can lookup               │   │
│  │                                                                       │   │
│  │  Privacy DHT:  key = hash(group_secret || peer_id)                   │   │
│  │                value = Encrypt(group_key, announcement)              │   │
│  │                                                                       │   │
│  │                ↓                                                      │   │
│  │                Only group members can compute lookup key             │   │
│  │                Only group members can decrypt announcement           │   │
│  │                                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      Announcement Structure                          │   │
│  │                                                                       │   │
│  │  ┌────────────────────────────────────────────────────────────────┐  │   │
│  │  │  Plaintext (visible to DHT nodes)                              │  │   │
│  │  │  ├── DHT Key (20 bytes, derived)                               │  │   │
│  │  │  └── Encrypted Blob (opaque to DHT)                            │  │   │
│  │  └────────────────────────────────────────────────────────────────┘  │   │
│  │                                                                       │   │
│  │  ┌────────────────────────────────────────────────────────────────┐  │   │
│  │  │  Encrypted Content (visible only to group)                     │  │   │
│  │  │  ├── Peer ID                                                   │  │   │
│  │  │  ├── Endpoints (IP:port pairs)                                 │  │   │
│  │  │  ├── Capabilities                                              │  │   │
│  │  │  ├── Timestamp                                                 │  │   │
│  │  │  └── Signature (verifies peer identity)                        │  │   │
│  │  └────────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Platform Architecture

### 6.1 Platform Abstraction

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Platform Abstraction                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      Platform Trait                                  │   │
│  │                                                                       │   │
│  │  trait Platform {                                                     │   │
│  │      type Socket: AsyncRead + AsyncWrite;                            │   │
│  │      type Timer: Future<Output = ()>;                                │   │
│  │      type Random: CryptoRng;                                          │   │
│  │                                                                       │   │
│  │      fn create_socket(&self) -> Result<Self::Socket>;                │   │
│  │      fn sleep(&self, duration: Duration) -> Self::Timer;             │   │
│  │      fn random(&self) -> Self::Random;                                │   │
│  │      fn now(&self) -> Instant;                                        │   │
│  │      fn features(&self) -> FeatureFlags;                             │   │
│  │  }                                                                    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                   Platform Implementations                           │   │
│  │                                                                       │   │
│  │  ┌───────────────────────────────────────────────────────────────┐   │   │
│  │  │                    Linux Native                                │   │   │
│  │  │                                                                 │   │   │
│  │  │  Features:                                                      │   │   │
│  │  │  ✓ AF_XDP kernel bypass (10-100 Gbps)                         │   │   │
│  │  │  ✓ io_uring async I/O                                          │   │   │
│  │  │  ✓ All transports available                                    │   │   │
│  │  │  ✓ Precise timing                                              │   │   │
│  │  │  ✓ mlock() for secure memory                                   │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  │                                                                       │   │
│  │  ┌───────────────────────────────────────────────────────────────┐   │   │
│  │  │                    Windows Native                              │   │   │
│  │  │                                                                 │   │   │
│  │  │  Features:                                                      │   │   │
│  │  │  ✓ IOCP async I/O                                              │   │   │
│  │  │  ✓ Standard socket transports                                  │   │   │
│  │  │  ✓ VirtualLock() for secure memory                            │   │   │
│  │  │  ✗ No kernel bypass (Winsock only)                            │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  │                                                                       │   │
│  │  ┌───────────────────────────────────────────────────────────────┐   │   │
│  │  │                    macOS Native                                │   │   │
│  │  │                                                                 │   │   │
│  │  │  Features:                                                      │   │   │
│  │  │  ✓ kqueue async I/O                                            │   │   │
│  │  │  ✓ Standard socket transports                                  │   │   │
│  │  │  ✓ mlock() for secure memory                                   │   │   │
│  │  │  ✗ No kernel bypass                                           │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  │                                                                       │   │
│  │  ┌───────────────────────────────────────────────────────────────┐   │   │
│  │  │                    WebAssembly (Browser)                       │   │   │
│  │  │                                                                 │   │   │
│  │  │  Features:                                                      │   │   │
│  │  │  ✓ WebSocket transport                                         │   │   │
│  │  │  ✓ WebRTC data channels (P2P)                                 │   │   │
│  │  │  ✓ Web Crypto API                                              │   │   │
│  │  │  ✗ No UDP                                                      │   │   │
│  │  │  ✗ No raw sockets                                             │   │   │
│  │  │  ✗ Limited timing precision                                   │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  │                                                                       │   │
│  │  ┌───────────────────────────────────────────────────────────────┐   │   │
│  │  │                    Embedded/IoT                                │   │   │
│  │  │                                                                 │   │   │
│  │  │  Features:                                                      │   │   │
│  │  │  ✓ Minimal memory footprint                                    │   │   │
│  │  │  ✓ no_std compatible core                                      │   │   │
│  │  │  ✓ Power-optimized modes                                       │   │   │
│  │  │  ✗ Limited crypto (software only)                             │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 6.2 Resource Profiles

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Resource Profiles                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  PERFORMANCE Profile                                                 │   │
│  │  ─────────────────────                                               │   │
│  │  Use case: Data centers, high-speed backups, media workflows        │   │
│  │                                                                       │   │
│  │  • Kernel bypass enabled (AF_XDP on Linux)                          │   │
│  │  • Large buffers (256 MB+)                                          │   │
│  │  • Jumbo frames (9000 MTU)                                          │   │
│  │  • Minimal padding overhead                                          │   │
│  │  • No cover traffic                                                  │   │
│  │  • Aggressive congestion control                                     │   │
│  │                                                                       │   │
│  │  Expected: 10-100 Gbps throughput, <1ms latency                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  BALANCED Profile                                                    │   │
│  │  ────────────────────                                                │   │
│  │  Use case: Desktop applications, general file sharing               │   │
│  │                                                                       │   │
│  │  • Standard sockets                                                  │   │
│  │  • Moderate buffers (64 MB)                                         │   │
│  │  • Standard MTU (1500)                                              │   │
│  │  • Moderate padding                                                  │   │
│  │  • Optional cover traffic                                            │   │
│  │  • Standard congestion control                                       │   │
│  │                                                                       │   │
│  │  Expected: 100 Mbps - 1 Gbps, 5-50ms latency                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  CONSTRAINED Profile                                                 │   │
│  │  ─────────────────────                                               │   │
│  │  Use case: Mobile devices, embedded systems, IoT                    │   │
│  │                                                                       │   │
│  │  • Minimal memory (16 MB max)                                       │   │
│  │  • Small chunks (16 KB)                                             │   │
│  │  • Power-optimized polling                                          │   │
│  │  • No compression (CPU constrained)                                 │   │
│  │  • No cover traffic                                                  │   │
│  │  • Conservative congestion control                                   │   │
│  │                                                                       │   │
│  │  Expected: 1-10 Mbps, battery-friendly                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  STEALTH Profile                                                     │   │
│  │  ─────────────────                                                   │   │
│  │  Use case: Censorship circumvention, hostile networks               │   │
│  │                                                                       │   │
│  │  • WebSocket or HTTP/2 transport                                    │   │
│  │  • Full padding (HTTPS distribution matching)                       │   │
│  │  • Timing obfuscation enabled                                        │   │
│  │  • Cover traffic enabled                                             │   │
│  │  • Probing resistance enabled                                        │   │
│  │  • Service fronting (if available)                                  │   │
│  │  • Entropy normalization                                             │   │
│  │                                                                       │   │
│  │  Expected: 10-50 Mbps, maximum privacy                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  METERED Profile                                                     │   │
│  │  ────────────────                                                    │   │
│  │  Use case: Cellular connections, satellite, pay-per-byte            │   │
│  │                                                                       │   │
│  │  • Aggressive compression                                            │   │
│  │  • Minimal padding                                                   │   │
│  │  • No cover traffic                                                  │   │
│  │  • Bandwidth budget enforcement                                      │   │
│  │  • Deduplication (content-addressed)                                │   │
│  │                                                                       │   │
│  │  Expected: Budget-limited, maximum efficiency                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Deployment Architectures

### 7.1 Peer-to-Peer Deployment

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Peer-to-Peer Deployment                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                          ┌─────────────────┐                               │
│                          │   Bootstrap     │                               │
│                          │   DHT Nodes     │                               │
│                          └────────┬────────┘                               │
│                                   │                                        │
│               ┌───────────────────┼───────────────────┐                    │
│               │                   │                   │                    │
│               ▼                   ▼                   ▼                    │
│        ┌─────────────┐     ┌─────────────┐     ┌─────────────┐            │
│        │   Peer A    │◄───►│   Peer B    │◄───►│   Peer C    │            │
│        │             │     │             │     │             │            │
│        │  (Desktop)  │     │  (Mobile)   │     │  (Server)   │            │
│        └─────────────┘     └─────────────┘     └─────────────┘            │
│               ▲                   ▲                   ▲                    │
│               │                   │                   │                    │
│               └───────────────────┼───────────────────┘                    │
│                                   │                                        │
│                          ┌────────▼────────┐                               │
│                          │  Relay Network  │                               │
│                          │   (Fallback)    │                               │
│                          └─────────────────┘                               │
│                                                                             │
│  Characteristics:                                                          │
│  • No central server                                                       │
│  • DHT for discovery                                                       │
│  • Direct connections preferred                                            │
│  • Relay fallback for restrictive NAT                                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Hub-and-Spoke Deployment

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      Hub-and-Spoke Deployment                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                          ┌─────────────────┐                               │
│                          │   Central Hub   │                               │
│                          │   (Server)      │                               │
│                          │                 │                               │
│                          │  • Always-on    │                               │
│                          │  • Public IP    │                               │
│                          │  • Coordinator  │                               │
│                          └────────┬────────┘                               │
│                                   │                                        │
│         ┌─────────────────────────┼─────────────────────────┐              │
│         │                         │                         │              │
│         ▼                         ▼                         ▼              │
│  ┌─────────────┐           ┌─────────────┐           ┌─────────────┐      │
│  │   Client A  │           │   Client B  │           │   Client C  │      │
│  │             │           │             │           │             │      │
│  │  (Behind    │           │  (Mobile    │           │  (Browser   │      │
│  │   NAT)      │           │   network)  │           │   client)   │      │
│  └─────────────┘           └─────────────┘           └─────────────┘      │
│                                                                             │
│  Characteristics:                                                          │
│  • Central coordination                                                    │
│  • Simpler NAT traversal (clients connect to hub)                         │
│  • Hub can relay for client-to-client                                     │
│  • Suitable for team/organization deployments                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.3 Mesh Group Deployment

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Mesh Group Deployment                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│         ┌─────────────┐                     ┌─────────────┐                │
│         │   Peer A    │◄───────────────────►│   Peer B    │                │
│         └──────┬──────┘                     └──────┬──────┘                │
│                │                                   │                       │
│                │         ┌─────────────┐           │                       │
│                └────────►│   Peer E    │◄──────────┘                       │
│                          │  (Optional  │                                   │
│                          │   relay)    │                                   │
│                          └──────┬──────┘                                   │
│                                 │                                          │
│                ┌────────────────┼────────────────┐                         │
│                │                │                │                         │
│         ┌──────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐                 │
│         │   Peer C    │◄►│   Peer D    │◄►│   Peer F    │                 │
│         └─────────────┘  └─────────────┘  └─────────────┘                 │
│                                                                             │
│  Group Features:                                                           │
│  • TreeKEM for group key management                                        │
│  • Forward secrecy for group communications                                │
│  • Scalable key updates (O(log n) messages)                               │
│  • Member add/remove with security guarantees                              │
│                                                                             │
│  Topologies:                                                               │
│  • Full mesh (small groups, <10 members)                                   │
│  • Tree (broadcast-heavy workloads)                                        │
│  • Gossip (large groups, eventual consistency)                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 8. Data Flow Architecture

### 8.1 Send Path

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Send Path                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Application Data                                                          │
│       │                                                                    │
│       ▼                                                                    │
│  ┌──────────────┐                                                          │
│  │  Chunking    │  Split into 256KB chunks, compute hashes                │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │  Compression │  Optional LZ4 (if beneficial)                           │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │   Framing    │  Add stream ID, sequence, offset                        │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │  Encryption  │  XChaCha20-Poly1305 with ratcheted key                  │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │   Padding    │  Match target size distribution                         │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │ Wire Format  │  Apply session-specific wire encoding                   │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │ Timing Queue │  Hold for timing distribution matching                  │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │    Pacer     │  Token bucket, BBR-derived rate                         │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │  Transport   │  UDP/TCP/WS/etc. send                                   │
│  └──────────────┘                                                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Receive Path

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             Receive Path                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Transport Receive                                                         │
│       │                                                                    │
│       ▼                                                                    │
│  ┌──────────────┐                                                          │
│  │ Wire Decode  │  Parse session-specific wire format                     │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │  CID Lookup  │  Find session by connection ID                          │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │  Decryption  │  XChaCha20-Poly1305 verify + decrypt                    │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │  Depadding   │  Strip random padding                                   │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────┐                                                          │
│  │Frame Parsing │  Extract type, stream ID, sequence                      │
│  └──────┬───────┘                                                          │
│         │                                                                  │
│         ├─────────────────────┬─────────────────────┐                      │
│         │                     │                     │                      │
│         ▼                     ▼                     ▼                      │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────────┐             │
│  │    DATA      │      │     ACK      │      │   CONTROL    │             │
│  │              │      │              │      │              │             │
│  │ • Reorder    │      │ • Update RTT │      │ • State      │             │
│  │   buffer     │      │ • Mark acked │      │   machine    │             │
│  │ • Decompress │      │ • Detect loss│      │              │             │
│  │ • Verify hash│      │ • Update     │      │              │             │
│  │ • Deliver    │      │   window     │      │              │             │
│  └──────────────┘      └──────────────┘      └──────────────┘             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 9. Extension Architecture

### 9.1 Extension Framework

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Extension Framework                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       Extension Points                               │   │
│  │                                                                       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│  │  │  Transport  │  │   Crypto    │  │ Obfuscation │  │   Frame     │  │   │
│  │  │ Extensions  │  │ Extensions  │  │ Extensions  │  │ Extensions  │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Extension Negotiation                             │   │
│  │                                                                       │   │
│  │  Phase 3 of handshake includes extension negotiation:                │   │
│  │                                                                       │   │
│  │  Initiator Extensions Offered:                                       │   │
│  │    [POST_QUANTUM, REAL_TIME_QOS, GROUP_V2, CUSTOM_PADDING]          │   │
│  │                                                                       │   │
│  │  Responder Extensions Accepted:                                      │   │
│  │    [POST_QUANTUM, REAL_TIME_QOS]                                     │   │
│  │                                                                       │   │
│  │  Session uses: POST_QUANTUM + REAL_TIME_QOS                         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Built-in Extensions                               │   │
│  │                                                                       │   │
│  │  POST_QUANTUM      │ Enable ML-KEM-768 hybrid key exchange          │   │
│  │  REAL_TIME_QOS     │ Enable QoS modes and FEC                       │   │
│  │  GROUP_V2          │ TreeKEM group key management                   │   │
│  │  CONTENT_ADDRESSED │ Merkle tree chunking for deduplication         │   │
│  │  RESUMPTION        │ Session resumption tickets                     │   │
│  │  MULTIPATH         │ Multiple simultaneous paths                    │   │
│  │  COMPRESSION       │ LZ4 compression support                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Custom Extension API                              │   │
│  │                                                                       │   │
│  │  trait Extension {                                                    │   │
│  │      fn extension_id(&self) -> u16;                                  │   │
│  │      fn negotiate(&self, params: &[u8]) -> Result<Vec<u8>>;         │   │
│  │      fn on_frame(&self, frame: &Frame) -> Result<Action>;           │   │
│  │      fn on_send(&self, data: &mut [u8]) -> Result<()>;              │   │
│  │      fn on_recv(&self, data: &mut [u8]) -> Result<()>;              │   │
│  │  }                                                                    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 10. Performance Architecture

### 10.1 Performance Optimization Layers

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Performance Optimization Layers                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Level 0: Algorithm Selection                     │   │
│  │                                                                       │   │
│  │  • XChaCha20: ~3x faster than AES-GCM without AES-NI                │   │
│  │  • BLAKE3: ~4x faster than SHA-256, parallelizable                  │   │
│  │  • BBR: Better than CUBIC for high-bandwidth paths                  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Level 1: Memory Management                       │   │
│  │                                                                       │   │
│  │  • Buffer pools: Avoid allocation in hot path                       │   │
│  │  • Zero-copy where possible                                          │   │
│  │  • Cache-aligned data structures                                     │   │
│  │  • NUMA-aware allocation (multi-socket)                             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Level 2: I/O Optimization                        │   │
│  │                                                                       │   │
│  │  • io_uring for async syscalls (Linux 5.1+)                         │   │
│  │  • Batched operations                                                │   │
│  │  • Vectored I/O (writev/readv)                                      │   │
│  │  • File system direct I/O (bypass page cache)                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Level 3: Kernel Bypass                           │   │
│  │                                                                       │   │
│  │  • AF_XDP: Zero-copy packet processing                              │   │
│  │  • eBPF: In-kernel packet filtering                                 │   │
│  │  • DPDK: Userspace network stack (alternative)                      │   │
│  │                                                                       │   │
│  │  Throughput comparison:                                              │   │
│  │  ┌────────────────────────────────────────────────────────────────┐ │   │
│  │  │  Standard sockets:  1-10 Gbps (CPU-bound)                      │ │   │
│  │  │  io_uring:          5-20 Gbps (reduced syscalls)               │ │   │
│  │  │  AF_XDP:            40-100 Gbps (kernel bypass)                │ │   │
│  │  └────────────────────────────────────────────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Level 4: Parallelism                             │   │
│  │                                                                       │   │
│  │  • Multi-threaded encryption (BLAKE3 parallelism)                   │   │
│  │  • Per-core packet processing                                        │   │
│  │  • Work-stealing schedulers                                          │   │
│  │  • Lock-free data structures                                         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 10.2 Benchmarking Targets

| Metric | Constrained | Balanced | Performance |
|--------|------------|----------|-------------|
| Throughput | 1-10 Mbps | 100 Mbps - 1 Gbps | 10-100 Gbps |
| Latency (P50) | <100ms | <20ms | <1ms |
| Latency (P99) | <500ms | <100ms | <10ms |
| CPU per Gbps | 100% | 20% | 2% |
| Memory | 16 MB | 64 MB | 256 MB |
| Connections | 10 | 1000 | 100,000 |

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 2.0.0 | 2026-01 | WRAITH Team | Initial v2 architecture |

---

*End of WRAITH Protocol v2 Architecture Overview*
