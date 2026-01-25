# WRAITH Protocol Agent Context

**W**ire-speed **R**esilient **A**uthenticated **I**nvisible **T**ransfer **H**andler

---

## 🛑 CRITICAL AGENT PROTOCOLS (ANTI-AMNESIA) 🛑

**These rules are ABSOLUTE. Violating them puts the project at risk.**

1.  **Additive Editing ONLY:**
    *   When updating documentation or code, you must **READ** the existing file first.
    *   Your output must contain **ALL** existing information + the **NEW** information.
    *   **NEVER** summarize, truncate, or "clean up" existing technical details (User Stories, Config Blocks, Code Snippets, Database Schemas) unless explicitly told to *delete* them.
    *   If a document is long, you must still output the **entire** updated content, not just the diff, unless using a specific patching tool.

2.  **The Superset Principle:**
    *   Version $N+1$ of a document must be a superset of Version $N$.
    *   If Version $N$ contains a specific byte-layout diagram, Version $N+1$ MUST contain it.
    *   If Version $N$ contains granular Acceptance Criteria, Version $N+1$ MUST contain them.

3.  **Negative Constraint Adherence:**
    *   If the user says "Do not remove X," treat X as immutable read-only data.
    *   Treat "Do not stub" as a command to fully implement logic/text.

---

## 1. Project Overview

WRAITH is a decentralized, secure file transfer protocol written in Rust, designed for high-performance, stealth, and resilience in contested environments.

**Core Capabilities:**
*   **Performance:** 10+ Gbps throughput using **AF_XDP** and **io_uring**.
*   **Security:** **Noise_XX** mutual authentication, **XChaCha20-Poly1305** encryption, **Elligator2** key encoding.
*   **Invisibility:** Protocol mimicry (TLS, DNS, ICMP) and timing obfuscation (Jitter).

**Current Status:**
*   **Core Protocol:** V1 specs defined. Transport and Crypto crates in active development.
*   **Tier 3 Clients:** Detailed specifications (v1.4.0+) complete. Implementation Phase 1 (Scaffolding/Core) active.

---

## 2. Architecture & Structure

The project is a Rust Workspace (`Cargo.toml` at root).

### 2.1 Crate Structure (`crates/`)
*   `wraith-core`: Wire format, session management, frame encoding.
*   `wraith-crypto`: Cryptographic primitives (Noise, AEAD, KDF, Ratcheting).
*   `wraith-transport`: Network layer (AF_XDP, io_uring, UDP/TCP).
*   `wraith-obfuscation`: Mimicry engines (DNS/TLS), Padding, Timing Jitter.
*   `wraith-discovery`: DHT, NAT traversal, Relay coordination.
*   `wraith-files`: File chunking, compression, integrity.
*   `wraith-cli`: Unified command-line interface.
*   `wraith-xdp`: eBPF/XDP programs (Kernel bypass).
*   `wraith-ffi`: Bindings for foreign languages (C/C++, Python).

### 2.2 Client Ecosystem

| Tier | Clients | Focus |
|:---:|:---|:---|
| **1** | `wraith-transfer`, `wraith-chat` | Core file transfer and secure messaging. |
| **2** | `wraith-sync`, `wraith-share` | Folder synchronization and ephemeral sharing. |
| **3** | `wraith-recon`, `wraith-redops` | **Advanced/Offensive Security.** (See below) |

---

## 3. Tier 3 Advanced Clients

### 3.1 WRAITH-Recon (Reconnaissance)
A high-performance network analysis and exfiltration assessment tool.
*   **Tech Stack:** Rust, AF_XDP, eBPF (libbpf-rs), Crossterm (TUI).
*   **Key Features:**
    *   **Passive Recon:** Zero-touch asset discovery via promiscuous capture.
    *   **Active Stealth:** Stateless scanning with timing obfuscation.
    *   **Protocol Mimicry:** DNS/DoH/ICMP tunneling for egress testing.
    *   **Governance:** **Signed RoE** (Rules of Engagement) required to run. **Kill Switch** via UDP broadcast.

### 3.2 WRAITH-RedOps (Adversary Emulation)
A comprehensive C2 framework for authorized red teaming.
*   **Tech Stack:**
    *   **Implant ("Spectre"):** `no_std` Rust, Position Independent Code (PIC), Indirect Syscalls, Sleep Mask.
    *   **Team Server:** Rust (`axum`), PostgreSQL, gRPC.
    *   **Client:** Tauri + React (TypeScript).
*   **Key Features:**
    *   **C2 Channels:** WRAITH-native (UDP), HTTPS, DNS, SMB (P2P).
    *   **Evasion:** Stack spoofing, AMSI/ETW patching, memory encryption.
    *   **Governance:** Hardcoded scope limits, Time-to-Live (TTL), Immutable Audit Logs.

---

## 4. Development Workflow

### Prerequisites
*   **Rust:** 1.75+ (Stable)
*   **Kernel:** Linux 6.2+ (Required for full AF_XDP/io_uring features)
*   **Tools:** `clang`, `llvm`, `libpcap-dev`, `protobuf-compiler`

### Common Commands

| Action | Command |
| :--- | :--- |
| **Build Workspace** | `cargo build --workspace` |
| **Run Recon (Dev)** | `cargo run -p wraith-cli --bin wraith-recon` |
| **Run Tests** | `cargo test --workspace` |
| **Lint** | `cargo clippy --workspace -- -D warnings` |
| **Build Docs** | `cargo doc --workspace --open` |
| **CI Checks** | `cargo xtask ci` |

---

## 5. Active Sprints & TODOs

### WRAITH-Recon (Phase 1)
*   **Focus:** Governance Core & AF_XDP Engine.
*   **Tasks:**
    *   Implement `RulesOfEngagement` struct and Ed25519 verification.
    *   Implement `SafetyController` with AtomicBool Kill Switch.
    *   Build `XdpSocket` abstraction and `kern.c` eBPF filter.

### WRAITH-RedOps (Phase 1)
*   **Focus:** Team Server & C2 Transport.
*   **Tasks:**
    *   Scaffold Team Server (Axum/Postgres).
    *   Define gRPC Protobufs (`c2.proto`).
    *   Implement `no_std` implant foundation (Panic handler, Entry point).

See `to-dos/clients/*.md` for granular acceptance criteria.
