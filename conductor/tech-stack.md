# Technology Stack

## Protocol Core (Rust)

| Component | Technology | Version / Details |
|-----------|-----------|-------------------|
| Language | Rust | Edition 2024, MSRV 1.88 |
| Async Runtime | [Tokio](https://tokio.rs/) | 1.35+ (full features) |
| Kernel Bypass | AF_XDP | Zero-copy packet I/O, UMEM, ring buffers |
| Async File I/O | [io-uring](https://crates.io/crates/io-uring) | 0.7, Linux-only |
| Cryptography (AEAD) | [chacha20poly1305](https://crates.io/crates/chacha20poly1305) | 0.10 (XChaCha20-Poly1305) |
| Cryptography (KX) | [x25519-dalek](https://crates.io/crates/x25519-dalek) | 2.0 (static_secrets) |
| Cryptography (Sig) | [ed25519-dalek](https://crates.io/crates/ed25519-dalek) | 2.1 (rand_core) |
| Cryptography (Hash) | [blake3](https://crates.io/crates/blake3) | 1.5 (tree-parallelizable) |
| Noise Protocol | [snow](https://crates.io/crates/snow) | 0.10 (Noise_XX pattern) |
| Key Zeroization | [zeroize](https://crates.io/crates/zeroize) | 1.7 (derive) |
| Key Encryption | [argon2](https://crates.io/crates/argon2) | 0.5 (Argon2id KDF) |
| Serialization | [bincode](https://crates.io/crates/bincode) | 2.0 (serde), [serde](https://crates.io/crates/serde) 1.0, [serde_json](https://crates.io/crates/serde_json) 1.0 |
| Concurrency | [dashmap](https://crates.io/crates/dashmap) | 6 (concurrent hash maps) |
| Concurrency | [crossbeam-queue](https://crates.io/crates/crossbeam-queue) | 0.3 (lock-free queues) |
| CLI | [clap](https://crates.io/crates/clap) | 4.4 (derive) |
| TUI | [ratatui](https://crates.io/crates/ratatui) | 0.26 (RedOps Console) |
| Logging | [tracing](https://crates.io/crates/tracing) | 0.1, [tracing-subscriber](https://crates.io/crates/tracing-subscriber) 0.3 (env-filter) |
| Error Handling | [thiserror](https://crates.io/crates/thiserror) | 2.0 (library errors), [anyhow](https://crates.io/crates/anyhow) 1.0 (application errors) |
| Testing | [proptest](https://crates.io/crates/proptest) | 1.4 (property-based), [criterion](https://crates.io/crates/criterion) 0.7 (benchmarks) |
| Sockets | [socket2](https://crates.io/crates/socket2) | 0.6 |
| RNG | [rand](https://crates.io/crates/rand) | 0.8, [rand_core](https://crates.io/crates/rand_core) 0.6 (getrandom), [rand_distr](https://crates.io/crates/rand_distr) 0.4 |

## Backend Services (RedOps Team Server)

| Component | Technology | Purpose |
|-----------|-----------|---------|
| API Framework | [Axum](https://github.com/tokio-rs/axum) | HTTP listener endpoints |
| gRPC | [Tonic](https://github.com/hyperium/tonic) | Operator-to-server RPC |
| Persistence | SQLite / in-memory | Implant database, campaign tracking |

## Frontend & Desktop Clients (9 Tauri Apps)

| Component | Technology | Version / Details |
|-----------|-----------|-------------------|
| Desktop Framework | [Tauri 2.0](https://tauri.app/) | Rust backend + web frontend |
| UI Library | [React 18](https://react.dev/) | Component-based UI |
| Build Tool | [Vite](https://vitejs.dev/) | Fast HMR development |
| Language | TypeScript | Strict mode |
| Styling | [Tailwind CSS](https://tailwindcss.com/) | Utility-first dark theme |
| State Management | [Zustand](https://zustand-demo.pmnd.rs/) | Lightweight stores |

**Desktop Clients:** WRAITH-Transfer, WRAITH-Chat, WRAITH-Sync, WRAITH-Share, WRAITH-Stream, WRAITH-Mesh, WRAITH-Publish, WRAITH-Vault, WRAITH-Recon

**Server Client:** WRAITH-RedOps Operator Client (Tauri 2.0)

## Mobile Clients

| Platform | Language | FFI Binding | Secure Storage | Push |
|----------|---------|-------------|----------------|------|
| Android | Kotlin + Jetpack Compose | JNI via cargo-ndk | Android Keystore (hardware-backed) | FCM |
| iOS | Swift + SwiftUI | UniFFI | iOS Keychain + Secure Enclave | APNs |

## RedOps Implant (Spectre)

| Component | Technology | Version / Details |
|-----------|-----------|-------------------|
| Language | Rust | no_std, no_main |
| OS APIs | Windows API (hashed), Syscalls (Linux) | Dynamic resolution, Indirect syscalls |
| Persistence | COM (ITaskService), Registry, Service | Native implementation (no shell fallback) |
| C2 | HTTP, SMB (Named Pipes), TCP (Socks) | Custom implementation |
| Scripting | PowerShell (CLR Hosting) | Unmanaged execution via .NET runtime |

## Architecture

- **Monorepo Structure:** Cargo workspace with 8 protocol crates, 9 Tauri desktop clients, 1 build automation crate (`xtask`), and 1 integration test crate.
- **Excluded Crates:** `wraith-xdp` (requires eBPF toolchain), `wraith-redops` (standalone builds).
- **Wire Format:** 8-byte Connection ID + encrypted payload + 16-byte auth tag (outer); 28-byte header + payload + random padding (inner).
- **Threading Model:** Thread-per-core with no locks in hot path. Sessions pinned to cores, NUMA-aware allocation.
- **Target Platform:** Linux 6.2+ (for AF_XDP, io_uring). Primary: x86_64. Secondary: aarch64.

## Build Profiles

| Profile | LTO | Codegen Units | Panic | Strip | Debug |
|---------|-----|---------------|-------|-------|-------|
| `dev` | off | default | unwind | no | true |
| `release` | thin | 1 | abort | true | no |
| `bench` | thin (inherited) | 1 | abort | true | true |

## Build & CI Commands

```bash
cargo build --workspace              # Development build
cargo build --release                # Optimized release build
cargo test --workspace               # Run all 2,140 tests
cargo clippy --workspace -- -D warnings  # Zero-warning lint policy
cargo fmt --all                      # Format all code
cargo fmt --all -- --check           # Verify formatting (CI)
cargo xtask ci                       # Full CI pipeline (fmt + clippy + test)
cargo xtask coverage --html          # Code coverage with HTML report
cargo doc --workspace --no-deps --open   # Generate and view documentation
cargo run -p wraith-cli -- --help    # Run the CLI
```
