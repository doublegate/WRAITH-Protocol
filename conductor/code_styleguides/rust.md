# Rust Style Guide -- WRAITH Protocol

This document defines the Rust coding standards and conventions for the WRAITH Protocol project. It is authoritative for all Rust code in the workspace: protocol crates (`crates/`), client backends (`clients/*/src-tauri/`), the FFI layer (`crates/wraith-ffi/`), and build automation (`xtask/`).

**Rust Edition:** 2024 | **MSRV:** 1.88 | **Toolchain:** Stable

*References: [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), [Rust Reference](https://doc.rust-lang.org/reference/), [Clippy Documentation](https://doc.rust-lang.org/clippy/)*

---

## Table of Contents

1. [Formatting and Layout](#1-formatting-and-layout)
2. [Naming Conventions](#2-naming-conventions)
3. [Type System](#3-type-system)
4. [Error Handling](#4-error-handling)
5. [Ownership and Borrowing](#5-ownership-and-borrowing)
6. [Async Programming](#6-async-programming)
7. [Unsafe Code](#7-unsafe-code)
8. [Cryptographic Code](#8-cryptographic-code)
9. [Performance](#9-performance)
10. [Testing](#10-testing)
11. [Documentation](#11-documentation)
12. [Dependencies](#12-dependencies)
13. [Concurrency and Parallelism](#13-concurrency-and-parallelism)
14. [FFI and Interop](#14-ffi-and-interop)
15. [Build and CI](#15-build-and-ci)
16. [Code Organization](#16-code-organization)
17. [Clippy and Linting](#17-clippy-and-linting)
18. [Let-Chain Patterns (Rust 2024)](#18-let-chain-patterns-rust-2024)

---

## 1. Formatting and Layout

### 1.1 Rustfmt

All code must be formatted with `rustfmt`. The CI pipeline enforces this with `cargo fmt --all -- --check`. No manual formatting overrides are permitted except via `rustfmt.toml` configuration.

If a project-level `rustfmt.toml` exists, it is the single source of truth for formatting. If no `rustfmt.toml` is present, the `rustfmt` defaults apply.

### 1.2 Line Length

- **Maximum line length:** 100 characters (rustfmt default).
- Strings and URLs may exceed this limit when breaking them would reduce readability.
- Comment lines should also respect the 100-character limit where practical.

### 1.3 Indentation

- **4 spaces** per indentation level. Tabs are forbidden.
- Continuation lines for function arguments, where clauses, and match arms align with the opening delimiter or use a single additional indentation level.

```rust
// Preferred: aligned with opening parenthesis
fn process_frame(
    data: &[u8],
    session: &mut Session,
    config: &SessionConfig,
) -> Result<Frame, FrameError> {
    // ...
}

// Also acceptable: single indentation continuation
fn process_frame(
    data: &[u8], session: &mut Session, config: &SessionConfig,
) -> Result<Frame, FrameError> {
    // ...
}
```

### 1.4 Blank Lines

- **Two blank lines** between top-level items (functions, structs, enums, impl blocks) in the same module.
- **One blank line** between methods within an `impl` block.
- **One blank line** between logical sections within a function body.
- **No blank lines** at the start or end of a block (`{}`).
- **No trailing whitespace** on any line.

### 1.5 Import Ordering and Grouping

Imports are organized into groups separated by blank lines, in the following order:

1. Standard library (`std`, `core`, `alloc`)
2. External crates (third-party dependencies)
3. Workspace crates (`wraith_core`, `wraith_crypto`, etc.)
4. Crate-local imports (`crate::`, `super::`, `self::`)

Within each group, imports are sorted alphabetically. Use nested imports to reduce line count, but avoid deeply nested trees.

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chacha20poly1305::aead::Aead;
use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use wraith_core::{Frame, FrameType, Session};
use wraith_crypto::SessionKeys;

use crate::error::TransportError;
use crate::worker::WorkerPool;
```

**Avoid wildcard imports** (`use module::*`) except in:
- Test modules (`use super::*` at the top of `mod tests`)
- Prelude re-exports from external crates where it is idiomatic (e.g., trait preludes)

### 1.6 Module Organization

- Prefer **file-named modules** (`session.rs`) over **directory modules** (`session/mod.rs`) for simple modules.
- Use **directory modules** (`node/mod.rs` or `node.rs` + `node/` directory) when a module has sub-modules.
- Module declarations in `lib.rs` or `mod.rs` should be ordered: `pub mod` first (alphabetical), then `mod` (alphabetical), then `pub use` re-exports.

```rust
// lib.rs
pub mod congestion;
pub mod error;
pub mod frame;
pub mod session;
pub mod stream;

mod internal_helpers;

pub use error::Error;
pub use frame::{Frame, FrameBuilder, FrameType};
pub use session::{Session, SessionConfig};
```

---

## 2. Naming Conventions

### 2.1 General Rules (RFC 430)

| Item | Convention | Example |
|------|-----------|---------|
| Crates | `snake_case` (hyphenated in Cargo, underscore in Rust) | `wraith-core` / `wraith_core` |
| Modules | `snake_case` | `frame`, `session_manager` |
| Types (structs, enums, traits) | `UpperCamelCase` | `FrameHeader`, `SessionState`, `Transport` |
| Enum variants | `UpperCamelCase` | `FrameType::StreamOpen` |
| Functions and methods | `snake_case` | `parse_header`, `update_rtt` |
| Local variables | `snake_case` | `recv_buffer`, `min_rtt` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_PAYLOAD_SIZE`, `FRAME_HEADER_SIZE` |
| Statics | `SCREAMING_SNAKE_CASE` | `PROTOCOL_VERSION` |
| Type parameters | Single uppercase letter or short `UpperCamelCase` | `T`, `E`, `KeyType` |
| Lifetimes | Short lowercase, typically `'a`, `'b` | `'a`, `'buf` |
| Feature flags | `snake_case` (no `use-` or `with-` prefix) | `simd`, `af_xdp` |

### 2.2 Acronym Handling

Acronyms in `UpperCamelCase` are treated as single words:

| Correct | Incorrect |
|---------|-----------|
| `Aead` | `AEAD` (in type names) |
| `Bbr` | `BBR` (in type names) |
| `DhtNode` | `DHTNode` |
| `IceAgent` | `ICEAgent` |
| `StunServer` | `STUNServer` |
| `XdpSocket` | `XDPSocket` |

Exception: Two-letter acronyms may remain uppercase when they are the entire type name and this is overwhelmingly conventional (e.g., `IO`). In WRAITH, prefer the lowercase-acronym form for consistency.

In `snake_case`, acronyms are fully lowercased:

```rust
fn parse_aead_header() { }
fn configure_bbr_state() { }
let ice_candidate = gather_candidates();
```

### 2.3 Conversion Methods

Follow the Rust API guidelines for conversion naming:

| Prefix | Cost | Ownership | Example |
|--------|------|-----------|---------|
| `as_` | Free | Borrows `&self` | `fn as_bytes(&self) -> &[u8]` |
| `to_` | Expensive | Borrows `&self`, returns new value | `fn to_vec(&self) -> Vec<u8>` |
| `into_` | Variable | Consumes `self` | `fn into_inner(self) -> T` |

```rust
impl Frame {
    /// View the frame payload as a byte slice (zero-cost).
    pub fn as_payload(&self) -> &[u8] { &self.payload }

    /// Convert frame to an owned byte vector (allocates).
    pub fn to_bytes(&self) -> Vec<u8> { /* ... */ }

    /// Consume the frame and return the payload buffer.
    pub fn into_payload(self) -> Vec<u8> { self.payload }
}
```

### 2.4 Getter and Setter Conventions

- Getters: Use the field name directly, not `get_` prefix.
- Setters: Use `set_` prefix.
- Boolean queries: Use `is_`, `has_`, or `can_` prefix.

```rust
impl BbrState {
    pub fn phase(&self) -> BbrPhase { self.phase }
    pub fn min_rtt(&self) -> Duration { self.min_rtt }
    pub fn bytes_in_flight(&self) -> u64 { self.bytes_in_flight }
    pub fn is_probing(&self) -> bool { self.phase == BbrPhase::ProbeRtt }

    pub fn set_cwnd(&mut self, cwnd: u64) { self.cwnd = cwnd; }
}
```

### 2.5 Iterator Method Names

| Method | Returns | Ownership |
|--------|---------|-----------|
| `iter()` | `Iter<'_, T>` | Borrows `&self` |
| `iter_mut()` | `IterMut<'_, T>` | Borrows `&mut self` |
| `into_iter()` | `IntoIter<T>` | Consumes `self` |

### 2.6 WRAITH-Specific Naming Patterns

- **Crate prefixes:** All workspace crates use the `wraith-` prefix in Cargo names and `wraith_` in Rust identifiers.
- **Error types:** Each crate defines a primary error enum at `crate::error::SomeError` (e.g., `CryptoError`, `FrameError`, `TransportError`).
- **Config types:** Configuration structs use the `Config` suffix (e.g., `SessionConfig`, `TransportConfig`, `NodeConfig`).
- **Builder types:** Builder structs use the `Builder` suffix (e.g., `FrameBuilder`).
- **State types:** State enums use the `State` suffix (e.g., `SessionState`, `StreamState`, `HandshakePhase`).

---

## 3. Type System

### 3.1 Newtype Pattern

Use newtypes to distinguish semantically different values of the same underlying type, especially for protocol identifiers and cryptographic material.

```rust
/// 8-byte connection identifier.
///
/// Connection IDs are derived from session keys and are opaque
/// to the network layer. They must not be confused with stream IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId([u8; 8]);

impl ConnectionId {
    /// Create a new connection ID from raw bytes.
    #[must_use]
    pub fn new(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    /// View the connection ID as a byte slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }
}
```

### 3.2 Builder Pattern

Use the builder pattern for complex configuration structs with many optional fields. Prefer the consuming builder style (each method takes `self` by value and returns `Self`).

```rust
pub struct FrameBuilder {
    frame_type: FrameType,
    flags: FrameFlags,
    stream_id: u16,
    sequence: u32,
    offset: u64,
    payload: Vec<u8>,
}

impl FrameBuilder {
    #[must_use]
    pub fn new(frame_type: FrameType) -> Self {
        Self {
            frame_type,
            flags: FrameFlags::new(),
            stream_id: 0,
            sequence: 0,
            offset: 0,
            payload: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_stream_id(mut self, id: u16) -> Self {
        self.stream_id = id;
        self
    }

    #[must_use]
    pub fn with_flags(mut self, flags: FrameFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn build(self) -> Result<Frame, FrameError> {
        // Validate and construct
    }
}
```

### 3.3 Enum Design

Enums are the preferred way to represent protocol state machines, frame types, and error hierarchies.

- Use `#[repr(u8)]` for enums that map to wire format byte values.
- Implement `TryFrom<u8>` for parsing from wire format.
- Derive `Debug, Clone, Copy, PartialEq, Eq, Hash` for value-like enums.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameType {
    Reserved = 0x00,
    Data = 0x01,
    Ack = 0x02,
    Control = 0x03,
    // ...
}

impl TryFrom<u8> for FrameType {
    type Error = FrameError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Err(FrameError::ReservedFrameType),
            0x01 => Ok(Self::Data),
            0x02 => Ok(Self::Ack),
            // ...
            _ => Err(FrameError::InvalidFrameType(value)),
        }
    }
}
```

### 3.4 Generic Constraints and Where Clauses

- Prefer `where` clauses over inline bounds when there are more than one or two constraints.
- Use `impl Trait` in argument position for simple cases.
- Use named generics when the type appears in multiple positions.

```rust
// Simple: inline bound
fn encrypt(data: &[u8], key: impl AsRef<[u8]>) -> Vec<u8> { /* ... */ }

// Complex: where clause
fn transfer_file<T, P>(
    transport: &T,
    path: P,
    config: &TransferConfig,
) -> Result<TransferId, TransferError>
where
    T: Transport + Send + Sync,
    P: AsRef<std::path::Path>,
{
    // ...
}
```

### 3.5 Trait Design

- **Small, focused traits:** Each trait should represent a single capability (e.g., `Transport`, `Encrypt`, `Discover`).
- **Object safety:** Design traits to be object-safe when runtime polymorphism may be needed. Avoid generic methods and `Self`-returning methods in such traits.
- **Default implementations:** Provide defaults where a reasonable default exists, but not for core operations that must be explicitly implemented.
- **Associated types vs generics:** Use associated types when there is exactly one natural implementation per type; use generics when multiple implementations are possible.

```rust
/// Network transport abstraction.
///
/// Implementations provide raw packet send/receive over different
/// backends (UDP, AF_XDP, QUIC).
pub trait Transport: Send + Sync {
    /// The error type returned by transport operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Send a packet to the given address.
    fn send_to(&self, data: &[u8], addr: std::net::SocketAddr) -> Result<usize, Self::Error>;

    /// Receive a packet, returning the data and source address.
    fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, std::net::SocketAddr), Self::Error>;
}
```

### 3.6 Derive Macros

Standard derive order (alphabetical within categories):

```rust
#[derive(
    Clone, Copy,           // Copy semantics
    Debug,                 // Debug formatting
    Default,               // Default construction
    PartialEq, Eq,         // Equality
    PartialOrd, Ord,       // Ordering
    Hash,                  // Hashing
    serde::Serialize, serde::Deserialize,  // Serialization
    zeroize::Zeroize, zeroize::ZeroizeOnDrop,  // Secret material
)]
```

Only derive traits that are actually needed. Do not derive `Clone` on types containing secret material unless `Zeroize` is also derived.

---

## 4. Error Handling

### 4.1 Error Type Hierarchy

Each crate defines its error types in an `error` module using `thiserror`:

```rust
// crates/wraith-core/src/error.rs
use thiserror::Error;

/// Core protocol errors.
#[derive(Debug, Error)]
pub enum Error {
    /// Frame parsing error.
    #[error("frame error: {0}")]
    Frame(#[from] FrameError),

    /// Session error.
    #[error("session error: {0}")]
    Session(#[from] SessionError),

    /// Cryptographic error.
    #[error("crypto error: {0}")]
    Crypto(#[from] wraith_crypto::CryptoError),
}
```

### 4.2 Library vs Application Errors

| Context | Crate | Pattern |
|---------|-------|---------|
| Library crates | `wraith-core`, `wraith-crypto`, etc. | `thiserror` with per-crate error enums |
| Application binaries | `wraith-cli`, `xtask` | `anyhow` for ergonomic error propagation |
| Tauri backends | `clients/*/src-tauri/` | `thiserror` for IPC-visible errors, `anyhow` internally |

### 4.3 Error Design Rules

- **Structured variants:** Error variants should carry relevant context, not just strings.
- **No stringly-typed errors:** Prefer structured enum variants over `String` payloads.
- **Descriptive messages:** `#[error(...)]` messages should be lowercase, concise, and include relevant values.
- **No secret data in errors:** Never include key material, nonces, or plaintext in error messages.

```rust
// Good: structured with context
#[error("frame too short: expected at least {expected}, got {actual}")]
TooShort { expected: usize, actual: usize },

// Bad: stringly-typed
#[error("{0}")]
Generic(String),

// Bad: secret data in error
#[error("decryption failed for key {key:?}")]
DecryptionFailed { key: [u8; 32] },  // NEVER do this
```

### 4.4 Result Propagation

- Use the `?` operator for propagation. Never use `.unwrap()` or `.expect()` in library code unless the invariant is provably upheld and documented.
- In application code (CLI, xtask), `.expect("descriptive message")` is acceptable for programmer errors that indicate bugs.
- Use `.map_err()` to convert between error types when `From` impls are not appropriate.

```rust
// Library code: always propagate
pub fn decrypt_frame(data: &[u8], key: &SessionKeys) -> Result<Frame, CryptoError> {
    let plaintext = aead_decrypt(data, &key.recv_key)?;
    let frame = Frame::parse(&plaintext)?;
    Ok(frame)
}

// Application code: expect is acceptable for provable invariants
fn main() -> anyhow::Result<()> {
    let config = load_config().context("failed to load configuration")?;
    // ...
}
```

### 4.5 Panic Policy

- **Library crates:** Must never panic. All fallible operations return `Result`.
- **Test code:** Panics are expected and natural (via `assert!`, `assert_eq!`, `unwrap()`).
- **Application binaries:** May panic on provable programmer errors (e.g., regex compilation of a literal pattern).
- **`debug_assert!`:** Use for invariants that are expensive to check. These are stripped in release builds.

---

## 5. Ownership and Borrowing

### 5.1 Function Parameter Guidelines

| Situation | Parameter Type | Example |
|-----------|---------------|---------|
| Read-only access to data | `&T` or `&[u8]` | `fn parse(data: &[u8])` |
| Need to mutate the value | `&mut T` | `fn update_rtt(&mut self, rtt: Duration)` |
| Small `Copy` types | `T` (by value) | `fn new(id: u16) -> Self` |
| Transferring ownership | `T` (by value) | `fn into_inner(self) -> Vec<u8>` |
| Flexible string input | `impl AsRef<str>` or `&str` | `fn load_config(path: &str)` |
| Flexible path input | `impl AsRef<Path>` | `fn send_file(path: impl AsRef<Path>)` |

### 5.2 Arc and Rc Usage

- Use `Arc<T>` for shared ownership across threads (always in async code).
- Use `Rc<T>` only in single-threaded contexts (never in async Tokio code).
- Prefer cloning `Arc` over complex lifetime management when ownership is shared.
- Wrap configuration in `Arc<Config>` and clone it into spawned tasks.

```rust
let config = Arc::new(NodeConfig::default());
let transport = Arc::new(UdpTransport::bind(addr).await?);

// Clone Arc handles for each spawned task
let config_clone = Arc::clone(&config);
let transport_clone = Arc::clone(&transport);
tokio::spawn(async move {
    worker_loop(config_clone, transport_clone).await;
});
```

### 5.3 Interior Mutability

| Primitive | Use Case | WRAITH Example |
|-----------|----------|----------------|
| `Mutex<T>` | Mutual exclusion (short critical sections) | Session state updates |
| `RwLock<T>` | Read-heavy, write-rare | Configuration, routing tables |
| `AtomicU64` / `AtomicBool` | Lock-free counters and flags | `bytes_in_flight`, `is_closed` |
| `DashMap<K, V>` | Concurrent hash map | Peer session registry |

- Prefer `tokio::sync::Mutex` in async code (it is cancellation-safe and does not block the runtime).
- Prefer `std::sync::Mutex` when the lock is held for a very short time and crossing await points is not needed.
- Prefer atomics over mutexes for simple counters and flags.
- Minimize the scope of lock guards. Do not hold a lock across `.await` points with `std::sync::Mutex`.

### 5.4 Lifetime Annotations

- Let the compiler infer lifetimes when possible (elision rules).
- Add explicit lifetimes when:
  - The function returns a reference.
  - Multiple reference parameters exist and the relationship is ambiguous.
  - The struct stores a reference.

```rust
/// A parsed frame that borrows its payload from the input buffer.
pub struct ParsedFrame<'buf> {
    pub header: FrameHeader,
    pub payload: &'buf [u8],
}

impl<'buf> ParsedFrame<'buf> {
    pub fn parse(data: &'buf [u8]) -> Result<Self, FrameError> {
        // Zero-copy: payload is a slice into `data`
    }
}
```

---

## 6. Async Programming

### 6.1 Tokio Runtime

WRAITH uses the [Tokio](https://tokio.rs/) multi-threaded runtime as its async executor. All async code in the project targets Tokio.

- Use `#[tokio::main]` for application entry points.
- Use `#[tokio::test]` for async tests.
- Never mix runtimes (e.g., do not use `async-std` or `smol` alongside Tokio).

### 6.2 Async Function Style

- Prefer `async fn` over `-> impl Future<...>` for most cases.
- Use `-> impl Future<...>` only when you need to name the future type or control `Send`/`Sync` bounds explicitly.
- Mark async functions `Send` when they may be spawned on the Tokio runtime.

```rust
// Preferred: async fn
pub async fn connect(addr: SocketAddr) -> Result<Session, TransportError> {
    // ...
}

// When needed: explicit future return for trait implementations
fn connect(&self, addr: SocketAddr) -> impl Future<Output = Result<Session, TransportError>> + Send {
    async move {
        // ...
    }
}
```

### 6.3 Cancellation Safety

- Be aware that any `.await` point is a potential cancellation point.
- Do not hold critical resources (file handles, lock guards) across `.await` unless using `tokio::sync::Mutex`.
- Use `tokio::select!` with care: cancelled branches drop their futures, which may lose work.
- Document cancellation safety properties in doc comments for public async APIs.

### 6.4 Channel Patterns

| Channel | Use Case | WRAITH Example |
|---------|----------|----------------|
| `mpsc` | Many producers, single consumer | Frame submission to worker |
| `broadcast` | One producer, many consumers | Event notification to operators |
| `oneshot` | Single response | Handshake completion signal |
| `watch` | Latest-value broadcast | Configuration updates |

```rust
use tokio::sync::{mpsc, oneshot, broadcast};

// Worker pool uses mpsc to receive frames
let (tx, mut rx) = mpsc::channel::<Frame>(1024);

// Event system uses broadcast for operator notifications
let (event_tx, _) = broadcast::channel::<Event>(256);

// Handshake uses oneshot for completion signal
let (done_tx, done_rx) = oneshot::channel::<SessionKeys>();
```

### 6.5 Task Spawning and Structured Concurrency

- Prefer `tokio::spawn` for independent background tasks.
- Use `tokio::task::JoinSet` to manage a dynamic set of tasks with structured lifecycle.
- Use `tokio::task::spawn_blocking` for CPU-intensive or blocking I/O operations.
- Always handle `JoinHandle` results. Do not ignore them.

```rust
use tokio::task::JoinSet;

let mut set = JoinSet::new();

for peer in peers {
    let transport = Arc::clone(&transport);
    set.spawn(async move {
        discover_peer(transport, peer).await
    });
}

// Collect results
while let Some(result) = set.join_next().await {
    match result {
        Ok(Ok(peer_info)) => { /* success */ }
        Ok(Err(e)) => { warn!("discovery failed: {e}"); }
        Err(e) => { warn!("task panicked: {e}"); }
    }
}
```

---

## 7. Unsafe Code

### 7.1 Policy

Unsafe code is permitted only when strictly necessary. All crate roots must include:

```rust
#![deny(unsafe_op_in_unsafe_fn)]
```

This ensures that even within `unsafe fn`, each unsafe operation must be wrapped in an explicit `unsafe {}` block with a safety comment.

### 7.2 Justified Uses in WRAITH

| Use Case | Crate | Justification |
|----------|-------|---------------|
| SIMD intrinsics | `wraith-core` (frame parsing) | Performance-critical header parsing using SSE2/NEON loads |
| AF_XDP / UMEM | `wraith-transport` | Kernel-bypass requires raw memory management |
| io_uring | `wraith-transport` | Async I/O ring buffer interactions |
| FFI boundary | `wraith-ffi` | C-compatible API requires raw pointer handling |
| `#[repr(C)]` types | `wraith-ffi` | Stable ABI for cross-language interop |

### 7.3 Safety Documentation

Every `unsafe` block must have a `// SAFETY:` comment immediately above or within it, explaining why the operation is sound:

```rust
// SAFETY: Caller ensures data.len() >= FRAME_HEADER_SIZE (28 bytes). x86_64 SSE2
// supports unaligned loads via _mm_loadu_si128. Pointers are derived from valid
// slice data and offsets are within bounds (ptr1 at 0, ptr2 at 12, both < 28).
unsafe {
    use core::arch::x86_64::*;
    let ptr1 = data.as_ptr() as *const __m128i;
    let _vec1 = _mm_loadu_si128(ptr1);
}
```

Every `unsafe fn` must have a `# Safety` section in its doc comment:

```rust
/// Parse frame header using SIMD instructions (x86_64 SSE2).
///
/// # Safety
///
/// Caller must ensure `data.len() >= FRAME_HEADER_SIZE` (28 bytes).
#[cfg(target_arch = "x86_64")]
pub(super) unsafe fn parse_header_simd(data: &[u8]) -> FrameHeader {
    // ...
}
```

### 7.4 Minimizing Unsafe Surface

- Keep `unsafe` blocks as small as possible. Extract safe abstractions around unsafe operations.
- Never use `unsafe` to bypass borrow checker rules. If the borrow checker rejects code, redesign the data flow.
- Prefer safe abstractions from vetted crates (e.g., `zerocopy`, `bytemuck`) over hand-rolled unsafe code for type punning.

---

## 8. Cryptographic Code

### 8.1 Constant-Time Operations

All operations involving secret data must be constant-time to prevent timing side-channel attacks. Use the `subtle` crate for comparisons and conditional operations.

```rust
use subtle::{Choice, ConstantTimeEq, ConditionallySelectable};

// Constant-time comparison
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    a.ct_eq(b).into()
}

// Timing-safe verification
#[must_use]
#[inline(never)]  // Prevent compiler from optimizing timing behavior
pub fn verify_16(a: &[u8; 16], b: &[u8; 16]) -> bool {
    ct_eq(a, b)
}
```

- Never use `==` to compare secret data (MACs, keys, nonces).
- Mark verification functions `#[inline(never)]` to prevent the compiler from introducing timing variations.

### 8.2 Zeroization

All types containing secret key material must derive `Zeroize` and `ZeroizeOnDrop`:

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SessionKeys {
    pub send_key: [u8; 32],
    pub recv_key: [u8; 32],
    pub chain_key: [u8; 32],
}
```

- Never implement `Clone` on secret types unless `Zeroize` is also derived.
- Do not log or debug-print secret material. Implement custom `Debug` that redacts secrets.
- Use `Zeroizing<T>` wrapper from the `zeroize` crate for temporary buffers containing secret data.

### 8.3 Key Material Handling

- Generate keys using cryptographically secure RNG (`rand::rngs::OsRng` or `getrandom`).
- Derive sub-keys using proper KDFs (HKDF-BLAKE3 in WRAITH).
- Store keys in fixed-size arrays (`[u8; 32]`), not `Vec<u8>`, to control memory layout.
- Password-based keys use Argon2id KDF with appropriate parameters.

### 8.4 Nonce Management

- XChaCha20-Poly1305 uses 192-bit (24-byte) nonces.
- Nonces must never be reused with the same key. WRAITH uses a combination of:
  - Monotonically increasing counters (within a session)
  - Random nonce generation (for initial messages)
- Counter-based nonces must be incremented atomically and checked for overflow.

### 8.5 Side-Channel Resistance

- Avoid data-dependent branches on secret data.
- Avoid data-dependent memory access patterns (no secret-indexed array lookups).
- Use `black_box()` in benchmarks to prevent dead-code elimination of crypto operations.
- Consider compiler and optimization barriers where necessary.

---

## 9. Performance

### 9.1 Zero-Copy Patterns

Frame parsing should reference existing buffers rather than copying:

```rust
/// Parse a frame from a buffer without copying the payload.
pub fn parse<'buf>(data: &'buf [u8]) -> Result<ParsedFrame<'buf>, FrameError> {
    if data.len() < FRAME_HEADER_SIZE {
        return Err(FrameError::TooShort {
            expected: FRAME_HEADER_SIZE,
            actual: data.len(),
        });
    }

    let header = parse_header(&data[..FRAME_HEADER_SIZE]);
    let payload = &data[FRAME_HEADER_SIZE..];

    Ok(ParsedFrame { header, payload })
}
```

### 9.2 SIMD Usage

WRAITH uses SIMD intrinsics for performance-critical frame parsing:

- Use `#[cfg(target_arch = "x86_64")]` and `#[cfg(target_arch = "aarch64")]` for platform-specific SIMD.
- Provide scalar fallbacks for all SIMD code paths.
- Gate SIMD behind a feature flag (`simd`) when it introduces compile-time complexity.
- Prefer SIMD loads/stores for bulk header parsing; fall back to scalar for field extraction.

### 9.3 Memory Allocation

- **Hot paths** (frame processing, encryption, packet send/receive): No heap allocations. Use stack buffers, slices, and pre-allocated pools.
- **Cold paths** (connection setup, configuration): Allocations are acceptable.
- Use `BufferPool` for reusable packet buffers.
- Use `Vec::with_capacity()` when the final size is known or estimable.
- Avoid `String` formatting in hot paths; prefer pre-formatted or fixed-size alternatives.

### 9.4 Cache-Friendly Data Structures

- Prefer contiguous arrays and `Vec<T>` over linked structures (`LinkedList`, `BTreeMap`) for frequently iterated data.
- Align data structures to cache line boundaries (64 bytes) when false sharing is a concern.
- Use struct-of-arrays layout for SIMD-friendly processing when beneficial.

### 9.5 Benchmarking

Use `criterion` for micro-benchmarks. Benchmarks live in `tests/benches/` or per-crate `benches/` directories.

```rust
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn bench_frame_parsing(c: &mut Criterion) {
    let data = generate_test_frame();
    c.bench_function("parse_frame_header", |b| {
        b.iter(|| Frame::parse(black_box(&data)))
    });
}

criterion_group!(benches, bench_frame_parsing);
criterion_main!(benches);
```

- Always use `black_box()` to prevent dead-code elimination.
- Run benchmarks with `cargo bench` (uses the `bench` profile with release optimizations + debug info).
- Compare before/after when making performance-related changes.

---

## 10. Testing

### 10.1 Unit Test Organization

Every module with logic should have a `#[cfg(test)]` module at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbr_initial_state() {
        let bbr = BbrState::new();
        assert_eq!(bbr.phase(), BbrPhase::Startup);
        assert_eq!(bbr.bytes_in_flight(), 0);
    }

    #[test]
    fn test_bbr_rtt_update() {
        let mut bbr = BbrState::new();
        bbr.update_rtt(Duration::from_millis(50));
        bbr.update_rtt(Duration::from_millis(30));
        assert_eq!(bbr.min_rtt(), Duration::from_millis(30));
    }
}
```

### 10.2 Test Naming Convention

Tests follow the pattern: `test_<unit>_<scenario>` or `test_<unit>_<condition>_<expected_outcome>`.

```rust
#[test] fn test_frame_parse_valid_data() { }
#[test] fn test_frame_parse_too_short() { }
#[test] fn test_ct_eq_different_lengths() { }
#[test] fn test_bbr_startup_to_drain_transition() { }
#[test] fn test_session_timeout_returns_error() { }
```

### 10.3 Async Tests

Use `#[tokio::test]` for tests that require an async runtime:

```rust
#[tokio::test]
async fn test_node_start_and_stop() {
    let node = Node::new_random().await.unwrap();
    node.start().await.unwrap();
    // ...
    node.stop().await.unwrap();
}
```

### 10.4 Property-Based Testing

Use `proptest` for testing protocol invariants:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn frame_roundtrip(frame_type in 1u8..=15, seq in any::<u32>(), payload_len in 0usize..8944) {
        let frame = build_test_frame(frame_type, seq, payload_len);
        let encoded = frame.encode();
        let decoded = Frame::parse(&encoded).unwrap();
        assert_eq!(decoded.header.sequence, seq);
    }

    #[test]
    fn encrypt_decrypt_roundtrip(plaintext in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let key = generate_test_key();
        let nonce = generate_test_nonce();
        let ciphertext = encrypt(&plaintext, &key, &nonce).unwrap();
        let recovered = decrypt(&ciphertext, &key, &nonce).unwrap();
        assert_eq!(plaintext, recovered);
    }
}
```

### 10.5 Test Helpers

- Test helpers live in a `test_utils` module or at the top of the `tests` module.
- Share test utilities across crates via a `#[cfg(test)]` public module or a dedicated `testutil` crate (if needed).
- Use `const` for test vectors (known-answer tests) derived from specifications.

### 10.6 What to Test

| Category | Required Tests |
|----------|---------------|
| Frame encoding/decoding | Round-trip, malformed input, boundary values, reserved types |
| Cryptographic operations | Known-answer vectors, round-trip, invalid keys/nonces, constant-time properties |
| Session state machine | All valid transitions, invalid transitions, timeout behavior |
| Congestion control | Phase transitions, RTT updates, window calculations |
| Error paths | Every `Result::Err` variant must have a test triggering it |
| Configuration | Default values, custom values, invalid values |

### 10.7 Benchmark Tests

Performance benchmarks use `criterion` (version 0.7) and reside alongside integration tests:

```bash
cargo bench                          # Run all benchmarks
cargo bench -- frame_parsing         # Run specific benchmark
```

---

## 11. Documentation

### 11.1 Doc Comment Style

- Use `///` for item-level documentation (functions, structs, enums, fields).
- Use `//!` for module-level and crate-level documentation.
- Use `//` for implementation comments (internal notes, algorithm explanations).

### 11.2 Required Sections

Public API doc comments should include the following sections where applicable:

```rust
/// Parse a frame from raw bytes.
///
/// Extracts the header fields and payload from a byte buffer,
/// performing validation on all fields including frame type,
/// payload length, and sequence number.
///
/// # Examples
///
/// ```
/// use wraith_core::Frame;
///
/// let data = [/* ... */];
/// let frame = Frame::parse(&data).unwrap();
/// assert_eq!(frame.frame_type, FrameType::Data);
/// ```
///
/// # Errors
///
/// Returns [`FrameError::TooShort`] if `data.len() < 28`.
/// Returns [`FrameError::InvalidFrameType`] for unrecognized type bytes.
///
/// # Panics
///
/// This function does not panic.
pub fn parse(data: &[u8]) -> Result<Frame, FrameError> {
    // ...
}
```

| Section | When Required |
|---------|--------------|
| Description | Always |
| `# Examples` | Public API functions (with `/// ``` ... ///` ``` code blocks) |
| `# Errors` | Functions returning `Result` |
| `# Panics` | Functions that can panic (or state explicitly that they do not) |
| `# Safety` | `unsafe fn` (mandatory) |

### 11.3 Module-Level Documentation

Every `lib.rs` and significant module should have `//!` documentation including:
- Purpose and scope
- Architecture diagram (ASCII art in code blocks)
- Cross-references to related modules
- Feature flags that affect the module

See `crates/wraith-core/src/lib.rs` for the canonical example.

### 11.4 Cross-References

Use intra-doc links to reference other items:

```rust
/// Handles frame parsing as described in [`Frame`].
///
/// See also [`FrameBuilder`] for constructing frames, and
/// [`FrameError`] for possible parsing failures.
```

### 11.5 Documentation Testing

All code examples in doc comments are compiled and tested by `cargo test --doc`. Ensure examples:
- Compile and run successfully.
- Use `# ` prefix for hidden setup lines.
- Use `no_run` attribute for examples that require runtime resources.
- Use `ignore` only as a last resort, with a comment explaining why.

---

## 12. Dependencies

### 12.1 Workspace Dependency Management

All shared dependencies are declared in the root `Cargo.toml` under `[workspace.dependencies]`. Crates reference workspace dependencies with `dep.workspace = true`:

```toml
# Root Cargo.toml
[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }
blake3 = "1.5"

# Crate Cargo.toml
[dependencies]
tokio = { workspace = true }
blake3 = { workspace = true }
```

This ensures consistent versions across all crates.

### 12.2 Version Pinning Strategy

- **Cryptographic crates:** Pin to exact minor versions. Security-critical code must not silently update.
- **Other dependencies:** Use `"major.minor"` semver ranges (e.g., `"1.35"` allows `1.35.x` patches).
- **Dev dependencies:** More relaxed pinning is acceptable.

### 12.3 Feature Flags

- Feature flags must be **additive** (enabling a feature never removes functionality).
- Name features descriptively without `use-` or `with-` prefixes: `simd`, `af_xdp`, `io_uring`.
- Gate platform-specific code behind `cfg` attributes, not feature flags, when the platform is the deciding factor.
- Document all feature flags in the crate-level doc comment.

### 12.4 Optional Dependencies

- Use `optional = true` for dependencies that are only needed under a feature flag.
- Re-export optional dependencies through public API only when the feature is enabled.

```toml
[features]
default = []
simd = []

[dependencies]
io-uring = { workspace = true, optional = true }

[target.'cfg(target_os = "linux")'.dependencies]
io-uring = { workspace = true }
```

### 12.5 Dependency Auditing

- Run `cargo audit` regularly to check for known vulnerabilities.
- Keep the dependency tree minimal. Before adding a new dependency, consider:
  1. Can this be implemented in a small amount of code?
  2. Is the crate well-maintained and audited?
  3. Does it introduce `unsafe` code?
  4. What is its transitive dependency footprint?

---

## 13. Concurrency and Parallelism

### 13.1 Thread-per-Core Model

WRAITH transport workers follow a thread-per-core architecture:

- Each worker thread is pinned to a specific CPU core.
- No locks in the hot path (packet send/receive).
- Sessions are pinned to cores for cache locality.
- Inter-worker communication uses lock-free queues (`crossbeam-queue`).

### 13.2 Lock-Free Data Structures

- Use `DashMap` for concurrent hash maps (peer/session registries).
- Use `crossbeam-queue` for multi-producer, multi-consumer queues.
- Use custom `SpscRingBuffer` and `MpscRingBuffer` for high-throughput data paths.

### 13.3 Atomic Operations

Use atomics for simple counters and flags:

```rust
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

pub struct SessionStats {
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    is_closed: AtomicBool,
}

impl SessionStats {
    pub fn record_send(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn close(&self) {
        self.is_closed.store(true, Ordering::Release);
    }

    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Acquire)
    }
}
```

**Ordering guidelines:**
- `Relaxed`: Counters and statistics (no ordering guarantees needed).
- `Acquire`/`Release`: Flags that gate access to other data.
- `SeqCst`: Avoid unless absolutely necessary (use `Acquire`/`Release` pairs instead).

### 13.4 NUMA-Aware Allocation

On multi-socket systems, allocate buffers on the NUMA node local to the worker core:

- The `numa` module in `wraith-transport` provides NUMA topology detection.
- Buffer pools are allocated per-NUMA node.
- Workers access only their local buffer pool.

### 13.5 Rayon for Data Parallelism

Use `rayon` for CPU-bound parallel computations that are not on the hot path:

- Tree hashing of file chunks (BLAKE3 is tree-parallelizable).
- Batch verification of signatures.
- File integrity verification.

Do not mix Rayon parallelism with the Tokio runtime on the same threads. Use `tokio::task::spawn_blocking` to bridge between the two.

---

## 14. FFI and Interop

### 14.1 C FFI (wraith-ffi crate)

The `wraith-ffi` crate provides a C-compatible API for external language integration:

```rust
/// Create a new WRAITH node.
///
/// Returns a pointer to the node, or null on failure.
/// The caller is responsible for calling `wraith_node_free` to release the node.
///
/// # Safety
///
/// The returned pointer must not be used after calling `wraith_node_free`.
#[no_mangle]
pub unsafe extern "C" fn wraith_node_new(config: *const WraithConfig) -> *mut WraithNode {
    // ...
}

/// Free a WRAITH node.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `wraith_node_new`, and must not have
/// been previously freed.
#[no_mangle]
pub unsafe extern "C" fn wraith_node_free(node: *mut WraithNode) {
    if !node.is_null() {
        // SAFETY: Caller guarantees node is valid and not previously freed.
        unsafe { drop(Box::from_raw(node)); }
    }
}
```

**FFI Rules:**
- All FFI types must be `#[repr(C)]` for stable ABI.
- Use `*const T` and `*mut T` for pointer parameters, never references.
- Return error codes (`i32`) or null pointers for failures. Never panic across FFI boundaries.
- Document ownership transfer semantics in `# Safety` sections.
- Use `catch_unwind` at FFI boundaries to prevent panics from crossing the boundary.

### 14.2 JNI (Android)

Android integration uses JNI via `cargo-ndk`:

- Rust functions are `#[no_mangle] pub extern "system"` with JNI calling convention.
- Use `jni` crate for type conversions between Java/Kotlin and Rust.
- Key material is stored in Android Keystore (hardware-backed when available).
- Error handling converts Rust `Result` to Java exceptions.

### 14.3 UniFFI (iOS)

iOS integration uses Mozilla's UniFFI:

- Define the API in a `.udl` file or using proc-macro attributes.
- UniFFI generates Swift bindings automatically.
- Key material is stored in iOS Keychain with Secure Enclave access when available.
- Async Rust functions are bridged to Swift `async`/`await`.

### 14.4 Error Handling Across FFI

- Never allow panics to unwind across FFI boundaries. Wrap top-level FFI functions in `std::panic::catch_unwind`.
- Use integer error codes or out-parameters for error reporting.
- Provide `wraith_last_error()` for retrieving detailed error messages.

---

## 15. Build and CI

### 15.1 Workspace Configuration

The Cargo workspace is defined in the root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/wraith-core",
    "crates/wraith-crypto",
    # ... all protocol crates
    "clients/wraith-transfer/src-tauri",
    # ... all client backends
    "xtask",
    "tests",
]
exclude = [
    "crates/wraith-xdp",      # Requires eBPF toolchain
    "clients/wraith-redops",   # Standalone builds
]
```

### 15.2 Cargo Profiles

| Profile | Purpose | Key Settings |
|---------|---------|-------------|
| `dev` | Development | `opt-level = 0`, `debug = true` |
| `dev` (deps) | Dependencies in dev mode | `opt-level = 3` (fast dependency builds) |
| `release` | Production | `lto = "thin"`, `codegen-units = 1`, `panic = "abort"`, `strip = true` |
| `bench` | Benchmarks | Inherits `release` + `debug = true` (for profiling symbols) |

### 15.3 Feature Flags

Feature flags are used sparingly and follow conventions:

- `simd`: Enable SIMD-accelerated frame parsing (x86_64 SSE2, aarch64 NEON).
- Platform features are handled via `cfg(target_os)` and `cfg(target_arch)`, not feature flags.

### 15.4 Conditional Compilation

```rust
// Platform-specific modules
#[cfg(target_os = "linux")]
pub mod af_xdp;

#[cfg(target_os = "linux")]
pub mod io_uring;

// Architecture-specific optimizations
#[cfg(target_arch = "x86_64")]
pub(super) fn parse_header_simd(data: &[u8]) -> FrameHeader { /* ... */ }

#[cfg(target_arch = "aarch64")]
pub(super) fn parse_header_simd(data: &[u8]) -> FrameHeader { /* ... */ }

// Feature-gated code
#[cfg(feature = "simd")]
mod simd_parse;
```

### 15.5 CI Pipeline

The CI pipeline executes via `cargo xtask ci`, which runs the following in order:

1. `cargo fmt --all --check` -- Verify formatting
2. `cargo clippy --workspace -- -D warnings` -- Zero-warning lint policy
3. `cargo test --all-features --workspace` -- Run all tests

All three checks must pass. Failures block merges to `main`.

### 15.6 Cross-Compilation

- Primary target: `x86_64-unknown-linux-gnu`
- Secondary target: `aarch64-unknown-linux-gnu`
- Mobile targets: `aarch64-linux-android`, `aarch64-apple-ios`
- Use `cross` crate for cross-compilation: `cross build --target aarch64-unknown-linux-gnu`
- Windows timeouts should be 2-3x Linux timeouts in CI matrices.

---

## 16. Code Organization

### 16.1 Crate Structure

Each protocol crate follows this structure:

```
crates/wraith-example/
  Cargo.toml
  src/
    lib.rs          # Crate root: module declarations, re-exports, crate docs
    error.rs        # Crate error types (thiserror)
    config.rs       # Configuration types (if applicable)
    module_a.rs     # Feature modules
    module_b.rs
    module_b/       # Sub-modules (when module_b is complex)
      sub_a.rs
      sub_b.rs
```

### 16.2 Public API Design

- **Minimal public surface:** Export only what consumers need. Use `pub(crate)` and `pub(super)` for internal visibility.
- **Re-export key types:** The crate root (`lib.rs`) should re-export the most commonly used types with `pub use`.
- **Flat imports for consumers:** Users should be able to write `use wraith_core::Frame;` rather than `use wraith_core::frame::Frame;`.

```rust
// lib.rs - canonical re-exports
pub use error::Error;
pub use frame::{Frame, FrameBuilder, FrameFlags, FrameType};
pub use session::{Session, SessionConfig, SessionState};
```

### 16.3 Module Files vs Directories

| Complexity | Approach | Example |
|------------|----------|---------|
| Simple module (<500 lines) | Single file | `congestion.rs` |
| Complex module (>500 lines, sub-modules) | Directory with `mod.rs` or paired file | `node/mod.rs` + `node/config.rs` |

When using the directory approach, `mod.rs` (or the parent file in the Rust 2018+ flat layout) should contain only module declarations, re-exports, and brief documentation. Implementation code lives in sub-module files.

### 16.4 Workspace Dependency Sharing

Protocol crates depend on each other through workspace path dependencies:

```toml
# crates/wraith-core/Cargo.toml
[dependencies]
wraith-crypto = { workspace = true }
```

Avoid circular dependencies. The dependency graph flows upward:
```
wraith-cli (binary)
  -> wraith-core
       -> wraith-crypto
  -> wraith-transport
       -> wraith-core
  -> wraith-obfuscation
       -> wraith-crypto
  -> wraith-discovery
       -> wraith-core
       -> wraith-crypto
  -> wraith-files
       -> wraith-core
       -> wraith-crypto
```

### 16.5 Constants and Magic Numbers

- Define all protocol constants in the appropriate crate with `pub const` and doc comments.
- No magic numbers in code. Every numeric literal in protocol logic should reference a named constant.
- Use `usize` for sizes and lengths, `u8` for wire format bytes, `u16`/`u32`/`u64` for protocol fields matching their wire format width.

```rust
/// Fixed frame header size in bytes.
pub const FRAME_HEADER_SIZE: usize = 28;

/// AEAD authentication tag size.
pub const AUTH_TAG_SIZE: usize = 16;

/// Connection ID size.
pub const CONNECTION_ID_SIZE: usize = 8;

/// Maximum payload size (MTU 9000 - header - auth tag).
const MAX_PAYLOAD_SIZE: usize = 8944;
```

---

## 17. Clippy and Linting

### 17.1 Crate-Level Lint Attributes

All crate roots (`lib.rs` or `main.rs`) must include:

```rust
#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(unsafe_op_in_unsafe_fn)]
```

### 17.2 CI Enforcement

The CI pipeline runs:

```bash
cargo clippy --workspace -- -D warnings
```

This treats all warnings (including rustc warnings like `dead_code` and `unused_imports`) as errors. Code must compile with zero warnings.

### 17.3 Allowed Exceptions

When a clippy lint must be suppressed, use an `#[allow(...)]` attribute with a comment explaining why:

```rust
/// Reserved for future sequence anomaly detection.
#[allow(dead_code)]
const MAX_SEQUENCE_DELTA: u32 = 1_000_000;
```

Common justified exceptions in WRAITH:

| Lint | Justification | Example |
|------|---------------|---------|
| `dead_code` | Constant reserved for future use | `MAX_SEQUENCE_DELTA` |
| `clippy::too_many_arguments` | Protocol functions naturally require many parameters | Frame construction |
| `clippy::type_complexity` | Complex async return types | Future combinators |
| `clippy::cast_possible_truncation` | Verified by preceding bounds check | Protocol field extraction |

### 17.4 Recommended Pedantic Lints

Consider enabling these pedantic lints on a per-crate basis:

```rust
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]  // Common in workspace crates
#![allow(clippy::must_use_candidate)]       // Too noisy for builders
```

Or cherry-pick specific pedantic lints:

```rust
#![warn(clippy::cloned_instead_of_copied)]
#![warn(clippy::explicit_iter_loop)]
#![warn(clippy::implicit_clone)]
#![warn(clippy::map_unwrap_or)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(clippy::unnested_or_patterns)]
```

### 17.5 Restriction Lints (Opt-In)

The following restriction lints are recommended for security-critical crates:

```rust
// In cryptographic crates (wraith-crypto)
#![warn(clippy::panic)]         // No panics in library code
#![warn(clippy::todo)]          // No unfinished code
#![warn(clippy::unwrap_used)]   // Force explicit error handling
```

Never enable `clippy::restriction` as a whole group.

### 17.6 #[must_use]

Apply `#[must_use]` to:
- Pure functions that return computed values.
- Builder methods that return the modified builder.
- Functions where ignoring the return value is almost certainly a bug.

```rust
#[must_use]
pub fn phase(&self) -> BbrPhase { self.phase }

#[must_use]
pub fn with_syn(mut self) -> Self {
    self.0 |= Self::SYN;
    self
}
```

---

## 18. Let-Chain Patterns (Rust 2024)

### 18.1 Overview

Rust 2024 Edition stabilizes let chains, allowing multiple `let` patterns to be combined in `if` and `while` conditions using `&&`. This eliminates nested `if let` blocks.

*Reference: [Let chains in if and while -- Rust Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/let-chains.html)*

### 18.2 When to Use

Use let chains to flatten nested pattern matching when multiple conditions must hold:

```rust
// Before (Rust 2021): nested if let
if let Some(session) = sessions.get(&peer_id) {
    if let SessionState::Established = session.state() {
        if session.is_ready() {
            send_frame(session, &frame)?;
        }
    }
}

// After (Rust 2024): let chain
if let Some(session) = sessions.get(&peer_id)
    && let SessionState::Established = session.state()
    && session.is_ready()
{
    send_frame(session, &frame)?;
}
```

### 18.3 When to Avoid

- **Complex logic:** If the chain has more than 3-4 conditions, consider extracting a helper function.
- **Side effects:** Let chains should contain pure pattern matches and boolean checks. Do not embed side-effecting expressions.
- **Readability:** If the flattened form is harder to read than the nested form, keep the nested form.

### 18.4 Temporary Scope Changes

Rust 2024 changes the drop order for temporaries in `if let` chains: temporaries are now dropped before the `else` branch executes. This is generally the desired behavior, but be aware of it when working with types that have significant `Drop` implementations (e.g., `MutexGuard`).

```rust
// Safe in Rust 2024: lock is dropped before the else branch
if let Ok(guard) = mutex.try_lock()
    && guard.is_ready()
{
    process(&guard);
}
// `guard` is dropped here, before any else branch
```

### 18.5 Migration

When updating existing code to use let chains:
1. Identify nested `if let` patterns.
2. Flatten using `&&` to combine the conditions.
3. Verify that the temporary drop order change does not affect behavior.
4. Run the test suite to confirm no regressions.

---

## Appendix: Quick Reference

### Common Patterns Checklist

- [ ] All crate roots have `#![warn(missing_docs)]`, `#![warn(clippy::all)]`, `#![deny(unsafe_op_in_unsafe_fn)]`
- [ ] Error types use `thiserror` with structured variants
- [ ] Public functions have `///` doc comments with `# Errors` and `# Panics` sections
- [ ] No `.unwrap()` in library code
- [ ] Secret data types derive `Zeroize` and `ZeroizeOnDrop`
- [ ] Cryptographic comparisons use `subtle::ConstantTimeEq`
- [ ] No heap allocations in hot paths
- [ ] All `unsafe` blocks have `// SAFETY:` comments
- [ ] All `unsafe fn` have `# Safety` doc sections
- [ ] Tests cover success and failure paths
- [ ] Imports follow the 4-group ordering convention
- [ ] No wildcard imports (except in test modules)
- [ ] Constants used instead of magic numbers
- [ ] `#[must_use]` on pure functions and builder methods

### Key Workspace Commands

```bash
cargo build --workspace              # Build everything
cargo test --workspace               # Run all 2,140 tests
cargo clippy --workspace -- -D warnings  # Lint with zero-warning policy
cargo fmt --all -- --check           # Check formatting
cargo xtask ci                       # Full CI pipeline
cargo doc --workspace --no-deps      # Generate documentation
cargo bench                          # Run benchmarks
cargo xtask coverage --html          # Coverage report
```

---

*This style guide is a living document. It evolves with the project and the Rust ecosystem. Propose changes via pull request with justification.*

*Sources: [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), [Clippy Documentation](https://doc.rust-lang.org/clippy/), [Rust Edition Guide -- Let Chains](https://doc.rust-lang.org/edition-guide/rust-2024/let-chains.html), [Tokio Documentation](https://tokio.rs/tokio/tutorial)*
