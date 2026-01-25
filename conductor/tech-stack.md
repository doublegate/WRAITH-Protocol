# Technology Stack

## Backend & Protocol
- **Primary Language:** Rust (1.88+, Edition 2024)
- **Asynchronous Runtime:** [Tokio](https://tokio.rs/)
- **API Framework:** [Axum](https://github.com/tokio-rs/axum) (HTTP) and [Tonic](https://github.com/hyperium/tonic) (gRPC)
- **Persistence:** [PostgreSQL](https://www.postgresql.org/) with [sqlx](https://github.com/launchbadge/sqlx) for compile-time verified queries
- **Kernel Acceleration:** AF_XDP (kernel bypass) and io_uring (async I/O)
- **Cryptography:** Noise Protocol (Noise_XX), XChaCha20-Poly1305, Ed25519, and BLAKE3

## Frontend & Desktop Clients
- **Framework:** [Tauri 2.0](https://tauri.app/)
- **UI Library:** [React](https://reactjs.org/)
- **Build Tool:** [Vite](https://vitejs.dev/)
- **Language:** TypeScript
- **Styling:** CSS (likely Tailwind, given common usage with Vite/React)

## Mobile Clients
- **Android:** Kotlin with Rust FFI bindings
- **iOS:** Swift with Rust FFI bindings

## Architecture
- **Monorepo Structure:** Rust Workspace for shared protocol crates and client-specific crates.
