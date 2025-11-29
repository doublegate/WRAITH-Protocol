# WRAITH Protocol Dependency Management

**Document Version:** 1.0.0
**Last Updated:** 2025-11-28
**Status:** Engineering Documentation

---

## Overview

This document describes the dependency management strategy for the WRAITH Protocol workspace. WRAITH uses Cargo for dependency management with careful attention to security, licensing, and version compatibility.

**Principles:**
- **Minimal Dependencies:** Only essential crates
- **Security First:** Regular audits, no known vulnerabilities
- **License Compatibility:** MIT/Apache-2.0 compatible
- **Version Pinning:** Lock file committed for reproducibility

---

## Core Dependencies

### Cryptography

**Noise Protocol:**
```toml
[dependencies]
snow = "0.9"  # Noise protocol framework
```
- **Purpose:** Noise_XX handshake implementation
- **License:** Apache-2.0
- **Why:** Well-audited, widely used, supports pattern we need

**AEAD Encryption:**
```toml
[dependencies]
chacha20poly1305 = "0.10"  # XChaCha20-Poly1305 AEAD
```
- **Purpose:** Session encryption
- **License:** MIT OR Apache-2.0
- **Why:** RustCrypto implementation, constant-time, well-tested

**Hashing:**
```toml
[dependencies]
blake3 = "1.5"  # BLAKE3 hash function
```
- **Purpose:** File hashing, key derivation
- **License:** CC0-1.0 OR Apache-2.0
- **Why:** Fastest cryptographic hash, parallelizable, official implementation

**Signatures:**
```toml
[dependencies]
ed25519-dalek = "2.1"  # Ed25519 signatures
curve25519-dalek = "4.1"  # Curve25519 operations
```
- **Purpose:** Long-term identity keys, DH key exchange
- **License:** BSD-3-Clause
- **Why:** Constant-time, well-audited, widely used

**Key Derivation:**
```toml
[dependencies]
hkdf = "0.12"  # HKDF key derivation
sha2 = "0.10"  # SHA-256 (for HKDF)
```
- **Purpose:** Deriving session keys from shared secret
- **License:** MIT OR Apache-2.0
- **Why:** Standard KDF, RustCrypto implementation

**Zeroization:**
```toml
[dependencies]
zeroize = { version = "1.7", features = ["derive"] }
```
- **Purpose:** Securely erase secrets from memory
- **License:** Apache-2.0 OR MIT
- **Why:** Essential for key security

### Networking

**Async Runtime:**
```toml
[dependencies]
tokio = { version = "1.36", features = ["full"] }
```
- **Purpose:** Async I/O, timers, task spawning
- **License:** MIT
- **Why:** Industry standard, excellent performance, active development

**UDP Sockets:**
```toml
[dependencies]
socket2 = "0.5"  # Low-level socket operations
```
- **Purpose:** Fine-grained socket control (reuse addr, recv buffer size)
- **License:** MIT OR Apache-2.0
- **Why:** Needed for UDP socket tuning

**AF_XDP (Linux-only):**
```toml
[target.'cfg(target_os = "linux")'.dependencies]
libbpf-sys = "1.3"  # libbpf bindings
libbpf-rs = "0.23"  # Safe libbpf wrapper
```
- **Purpose:** Kernel bypass via AF_XDP
- **License:** LGPL-2.1 OR BSD-2-Clause (libbpf-sys), BSD-2-Clause (libbpf-rs)
- **Why:** Official Rust bindings for libbpf

**io_uring (Linux-only):**
```toml
[target.'cfg(target_os = "linux")'.dependencies]
io-uring = "0.7"  # io_uring async I/O
```
- **Purpose:** High-performance file I/O
- **License:** MIT OR Apache-2.0
- **Why:** Fastest I/O interface on Linux

### DHT & Discovery

**DHT Implementation:**
```toml
[dependencies]
libp2p = { version = "0.53", default-features = false, features = ["kad", "noise", "dns"] }
```
- **Purpose:** Kademlia DHT for peer discovery
- **License:** MIT
- **Why:** Battle-tested P2P stack, modular design

**Alternative (minimal DHT):**
```toml
# If not using libp2p
[dependencies]
tiny-kademlia = { git = "https://github.com/wraith/tiny-kademlia", branch = "main" }
```
- **Purpose:** Lightweight Kademlia implementation
- **License:** MIT
- **Why:** Smaller footprint, custom privacy features

### Serialization

```toml
[dependencies]
bincode = "1.3"  # Binary serialization
serde = { version = "1.0", features = ["derive"] }
```
- **Purpose:** Frame serialization, configuration
- **License:** MIT
- **Why:** Compact binary format, fast, widely used

### Error Handling

```toml
[dependencies]
thiserror = "1.0"  # Error derive macros
anyhow = "1.0"  # Flexible error handling (CLI only)
```
- **Purpose:** Ergonomic error definitions and propagation
- **License:** MIT OR Apache-2.0
- **Why:** Simplifies error handling, reduces boilerplate

### Logging & Tracing

```toml
[dependencies]
tracing = "0.1"  # Structured logging
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
```
- **Purpose:** Debug and production logging
- **License:** MIT
- **Why:** Structured, async-aware, filterable

### CLI (wraith-cli only)

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
indicatif = "0.17"  # Progress bars
console = "0.15"  # Terminal formatting
```
- **Purpose:** Argument parsing, user interface
- **License:** MIT OR Apache-2.0
- **Why:** Feature-rich, ergonomic, widely adopted

---

## Development Dependencies

### Testing

```toml
[dev-dependencies]
tokio-test = "0.4"  # Tokio test utilities
proptest = "1.4"  # Property-based testing
criterion = "0.5"  # Benchmarking
```

### Tooling

```toml
[dev-dependencies]
tempfile = "3.10"  # Temporary files for tests
hex = "0.4"  # Hex encoding for debugging
```

---

## Dependency Audit

### Regular Audits

**Using `cargo-audit`:**
```bash
# Install cargo-audit
cargo install cargo-audit

# Run security audit
cargo audit

# Fix advisories (if available)
cargo audit fix
```

**CI Integration:**
```yaml
# .github/workflows/security.yml
- name: Security Audit
  run: cargo audit --deny warnings
```

### Dependency Updates

**Check for outdated dependencies:**
```bash
# Install cargo-outdated
cargo install cargo-outdated

# Check for updates
cargo outdated

# Update dependencies (respecting Cargo.toml constraints)
cargo update

# Upgrade to latest (breaking changes possible)
cargo upgrade  # Requires cargo-edit
```

**Update Strategy:**
- **Patch versions:** Update immediately (bug fixes)
- **Minor versions:** Update regularly (new features, backward compatible)
- **Major versions:** Review changelog, test thoroughly before updating

### License Compliance

**Check licenses:**
```bash
# Install cargo-license
cargo install cargo-license

# List all dependency licenses
cargo license

# Verify compatibility
cargo license --json | jq '.[] | select(.license | contains("GPL"))'
```

**Acceptable Licenses:**
- MIT
- Apache-2.0
- BSD-2-Clause, BSD-3-Clause
- ISC
- CC0-1.0 (public domain)

**Unacceptable Licenses:**
- GPL-3.0 (copyleft, incompatible with MIT/Apache-2.0 dual licensing)
- AGPL (network copyleft)
- Proprietary licenses

---

## Version Pinning Strategy

### Cargo.lock

**Commit Cargo.lock to version control:**
```bash
git add Cargo.lock
git commit -m "chore: update Cargo.lock"
```

**Why:** Ensures reproducible builds across environments.

### Cargo.toml Versioning

**Use caret requirements (default):**
```toml
[dependencies]
tokio = "1.36"  # Equivalent to "^1.36" (>=1.36.0, <2.0.0)
```

**Tilde requirements for conservative updates:**
```toml
[dependencies]
snow = "~0.9.3"  # >=0.9.3, <0.10.0
```

**Exact versions for critical dependencies:**
```toml
[dependencies]
ed25519-dalek = "=2.1.0"  # Exactly 2.1.0 (cryptography stability)
```

---

## Workspace Dependencies

**Shared dependencies in workspace Cargo.toml:**
```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/wraith-core",
    "crates/wraith-crypto",
    "crates/wraith-transport",
    # ...
]

[workspace.dependencies]
tokio = { version = "1.36", features = ["full"] }
tracing = "0.1"
thiserror = "1.0"
blake3 = "1.5"

# Crates can then use: tokio = { workspace = true }
```

**Benefits:**
- Consistent versions across crates
- Easier updates (change once)
- Reduced duplication

---

## Dependency Reduction

### Analyze Dependency Tree

```bash
# Install cargo-tree
cargo install cargo-tree

# Show dependency tree
cargo tree

# Show specific dependency
cargo tree -p tokio

# Show duplicate dependencies
cargo tree --duplicates
```

### Feature Flags

**Minimize features:**
```toml
[dependencies]
# BAD: Includes all features (bloat)
tokio = { version = "1.36", features = ["full"] }

# GOOD: Only needed features
tokio = { version = "1.36", features = ["net", "rt-multi-thread", "time"] }
```

### Optional Dependencies

```toml
[dependencies]
# Always included
blake3 = "1.5"

# Optional dependencies
libbpf-rs = { version = "0.23", optional = true }

[features]
af-xdp = ["libbpf-rs"]
```

**Usage:**
```bash
# Build without AF_XDP
cargo build --no-default-features

# Build with AF_XDP
cargo build --features af-xdp
```

---

## Reproducible Builds

### Rust Version

**Specify minimum Rust version:**
```toml
[package]
rust-version = "1.75"
```

**CI uses specific version:**
```yaml
- uses: actions-rs/toolchain@v1
  with:
    toolchain: 1.75.0
    override: true
```

### Build Environment

**Document build requirements:**
```markdown
# Building WRAITH

## Requirements
- Rust 1.75.0+
- Linux kernel 6.2+ (for XDP features)
- libbpf-dev, libelf-dev, clang

## Build
cargo build --release
```

---

## Troubleshooting

### Issue: Dependency Conflict

**Symptom:**
```
error: failed to select a version for `...`
```

**Solution:**
```bash
# Update Cargo.lock
cargo update

# Or update specific dependency
cargo update -p problematic-dep

# Check for duplicates
cargo tree --duplicates
```

### Issue: Outdated Dependencies

**Symptom:**
```
warning: package `foo` uses deprecated API
```

**Solution:**
```bash
# Check for updates
cargo outdated

# Update dependencies
cargo update

# Upgrade to latest versions
cargo upgrade
```

---

## Best Practices

1. **Audit regularly:** Run `cargo audit` in CI
2. **Update promptly:** Security patches should be applied immediately
3. **Test updates:** Run full test suite after dependency updates
4. **Review changelogs:** Understand breaking changes before major version upgrades
5. **Minimize dependencies:** Question every new dependency
6. **Pin critical deps:** Use exact versions for cryptography
7. **Document reasons:** Comment why each dependency is needed

---

## See Also

- [Development Guide](development-guide.md)
- [Coding Standards](coding-standards.md)
- [Testing Strategy](../testing/testing-strategy.md)
