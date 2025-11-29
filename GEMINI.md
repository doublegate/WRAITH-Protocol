# WRAITH Protocol

**W**ire-speed **R**esilient **A**uthenticated **I**nvisible **T**ransfer **H**andler

## Project Overview

WRAITH is a decentralized, secure file transfer protocol written in Rust. It is designed for:
-   **High Performance:** 10+ Gbps throughput using AF_XDP and io_uring.
-   **Strong Security:** Mutual authentication, perfect forward secrecy, and traffic analysis resistance.
-   **Invisibility:** Protocol mimicry and timing obfuscation to resist censorship.

**Current Status:** Initial scaffolding complete. Core modules in development. Advanced Tier 3 clients (`wraith-recon`, `wraith-redops`) fully specified.

## Architecture & Structure

The project is organized as a Rust workspace with the following crates:

-   `crates/wraith-core`: Core protocol logic, frame encoding, session management.
-   `crates/wraith-crypto`: Cryptographic primitives (Noise handshake, AEAD, Elligator2).
-   `crates/wraith-transport`: Network transport layer (AF_XDP, io_uring, UDP sockets).
-   `crates/wraith-obfuscation`: Traffic analysis resistance (padding, timing, cover traffic).
-   `crates/wraith-discovery`: Peer discovery (DHT), NAT traversal, and relaying.
-   `crates/wraith-files`: File chunking, integrity verification.
-   `crates/wraith-cli`: Command-line interface.
-   `crates/wraith-xdp`: eBPF/XDP programs for kernel bypass (Linux-only).
-   `xtask`: Build automation and CI tasks.

### Client Ecosystem
-   **Tier 1:** `wraith-transfer`, `wraith-chat`
-   **Tier 2:** `wraith-sync`, `wraith-share`
-   **Tier 3 (Advanced):**
    -   `wraith-recon`: AF_XDP-based passive/active reconnaissance and DLP assessment tool.
    -   `wraith-redops`: Adversary emulation platform with memory-resident implants (`no_std` Rust).

## CRITICAL AGENT PROTOCOLS (ANTI-AMNESIA)

**These rules exist to prevent data loss during iterative document updates. Violating them puts the project at risk.**

1.  **Additive Editing ONLY:**
    *   When asked to "update" or "enhance" a document, you must **READ** the existing file first.
    *   Your output must contain **ALL** existing information + the **NEW** information.
    *   **NEVER** summarize, truncate, or "clean up" existing technical details (User Stories, Config Blocks, Code Snippets, Database Schemas) unless explicitly told to *delete* them.
    *   If a document is long, you must still output the **entire** updated content, not just the diff, unless using a tool specifically designed for patching.

2.  **The Superset Principle:**
    *   Version $N+1$ of a document must be a superset of Version $N$.
    *   If Version $N$ contains a specific byte-layout diagram, Version $N+1$ MUST contain it.
    *   If Version $N$ contains granular Acceptance Criteria, Version $N+1$ MUST contain them.

3.  **Negative Constraint Adherence:**
    *   If the user says "Do not remove X," treat X as immutable read-only data that must be copied verbatim to the new version.
    *   Treat "Do not stub" as a command to fully implement logic/text, never leaving `// ... rest of code` or `(same as before)` in file outputs.

## Development Workflow

### Prerequisites
-   **Rust:** 1.75+ (2021 edition)
-   **OS:** Linux 6.2+ (required for AF_XDP and io_uring features)
-   **Arch:** x86_64 or aarch64

### Common Commands

| Action | Command |
| :--- | :--- |
| **Build** | `cargo build --workspace` |
| **Release Build** | `cargo build --release` |
| **Test** | `cargo test --workspace` |
| **Lint** | `cargo clippy --workspace -- -D warnings` |
| **Format** | `cargo fmt --all` |
| **CI Checks** | `cargo xtask ci` |
| **Docs** | `cargo doc --workspace --open` |
| **Run CLI** | `cargo run -p wraith-cli -- --help` |

## Key Technical Details

-   **Cryptography:** X25519 (Elligator2), XChaCha20-Poly1305, BLAKE3, Noise_XX.
-   **Wire Format:** 8B CID + Encrypted Payload + 16B Auth Tag.
-   **Threading Model:** Thread-per-core, lock-free hot path, NUMA-aware.
-   **Advanced Networking:**
    -   **AF_XDP:** Zero-copy packet capture and injection (`wraith-recon`).
    -   **eBPF:** In-kernel packet filtering.
    -   **no_std:** Freestanding Rust for implants (`wraith-redops`).

## Coding Standards & Conventions

-   **Style:** Follow standard Rust conventions (`rustfmt`).
-   **Commits:** Conventional Commits (`feat:`, `fix:`, `docs:`, etc.).
-   **Safety:** No `unsafe` code in crypto paths. Memory safety is a priority.