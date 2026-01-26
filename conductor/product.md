# Product Definition -- WRAITH Protocol

**Version:** 2.2.5 | **Status:** Production

## Initial Concept

A decentralized secure file transfer protocol optimized for high-throughput, low-latency operation with strong security guarantees and traffic analysis resistance.

## Core Value Proposition

WRAITH Protocol provides wire-speed, secure, and invisible file transfer capabilities through a six-layer architecture:

1. **Network** -- UDP, raw sockets, covert channels (DNS, ICMP, HTTPS, WebSocket mimicry)
2. **Kernel Acceleration** -- AF_XDP (zero-copy packet I/O), io_uring (async file I/O), NUMA-aware allocation
3. **Obfuscation** -- Elligator2 key encoding, 5 padding strategies, 5 timing profiles, protocol mimicry (TLS 1.3, DNS-over-HTTPS, WebSocket)
4. **Crypto Transport** -- Noise_XX mutual authentication, XChaCha20-Poly1305 AEAD (192-bit nonce), Double Ratchet forward secrecy
5. **Session** -- Stream multiplexing, flow control, BBR congestion control, connection migration (RFC 8445 ICE)
6. **Application** -- File chunking (BLAKE3 tree hashing), integrity verification, reassembly

The protocol leverages kernel-bypass networking (AF_XDP) and modern cryptography (Noise_XX, Double Ratchet) to ensure data confidentiality, integrity, and forward secrecy while minimizing latency. Its traffic obfuscation techniques make it resistant to deep packet inspection and traffic analysis.

## Key Features

- **High Performance:** File chunking at 14.85 GiB/s, tree hashing at 4.71 GiB/s, verification at 4.78 GiB/s, reassembly at 5.42 GiB/s. 10-40 Gbps throughput with kernel bypass; sub-millisecond latency with AF_XDP.
- **Strong Security:** End-to-end encryption using XChaCha20-Poly1305 AEAD, Noise_XX mutual authentication, X25519 key exchange with Elligator2 encoding, Ed25519 signatures, and perfect forward secrecy via ratcheting (every 2 minutes or 1M packets). Zero known vulnerabilities across 295 dependencies.
- **Traffic Obfuscation & Evasion:** Traffic analysis resistance through protocol mimicry (TLS 1.3, DNS-over-HTTPS, WebSocket), 5 padding strategies, and 5 timing jitter profiles. RedOps client provides advanced evasion features including ROP-based memory sleep masking, indirect syscalls (Halo's Gate), stack spoofing, unmanaged PowerShell execution via CLR, and multi-platform process injection (Reflective, Hollowing, Hijack). Supports peer-to-peer mesh C2 routing.
- **Decentralized Discovery:** Privacy-preserving peer discovery using secure Kademlia DHT, STUN for NAT type detection, full RFC 8445 ICE signaling (candidate gathering, connectivity checks, nominated pairs), and relay fallback for NAT traversal.
- **Resilience:** Stateless recovery from packet loss, BBR congestion control, connection migration with path challenge/response, and multi-path support.
- **Cross-Platform Ecosystem:** 12 client applications spanning desktop (Tauri 2.0 + React), mobile (Android/Kotlin + JNI, iOS/Swift + UniFFI), and server environments. Includes file transfer, E2EE messaging with voice/video, file synchronization, swarm sharing, media streaming, mesh networking, censorship-resistant publishing, distributed secret storage, network reconnaissance, and red team operations.

## Target Audience

- **Privacy-Conscious Individuals:** Users requiring secure, private file sharing without reliance on centralized cloud providers.
- **Security Professionals:** Red teams and security researchers needing covert channel capabilities and network reconnaissance tools (WRAITH-Recon, WRAITH-RedOps).
- **Enterprise & Infrastructure:** Organizations needing high-throughput, secure data transfer solutions for internal or cross-boundary flows.
- **Developers:** Builders integrating secure P2P transfer capabilities into their own applications via the `wraith-ffi` C-compatible API or direct Rust crate consumption.

## Success Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Throughput | >9 Gbps on 10GbE links | 14.85 GiB/s chunking (achieved) |
| Handshake Latency | <50ms | Sub-millisecond with AF_XDP |
| Security Rating | Grade A+ (zero critical vulns) | Zero vulnerabilities (295 deps) |
| Client Deployment | All 12 clients stable | 12/12 complete |
| Test Pass Rate | 100% | 2,140 passing (16 ignored) |
| Quality Score | >95/100 | 98/100 |
| Technical Debt | <5% | 2.5% |
| Clippy Warnings | Zero | Zero |
