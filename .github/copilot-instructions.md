# Copilot Agent Instructions for WRAITH Protocol

## Project Summary

WRAITH Protocol is a decentralized secure file transfer protocol written in Rust. It's a multi-crate workspace targeting Linux with AF_XDP kernel bypass and io_uring for high-performance networking.

**Status:** Scaffolded implementation - core modules have structure but many TODOs for full implementation.

## Quick Reference

| Task | Command |
|------|---------|
| Build | `cargo build --workspace` |
| Test | `cargo test --workspace` |
| Lint | `cargo clippy --workspace -- -D warnings` |
| Format check | `cargo fmt --all -- --check` |
| Format fix | `cargo fmt --all` |
| Main CI checks | `cargo run -p xtask -- ci` |
| CLI help | `cargo run -p wraith-cli -- --help` |
| Docs | `cargo doc --workspace --no-deps` |

**IMPORTANT:** The xtask command requires `cargo run -p xtask --` NOT `cargo xtask`. The latter will fail with "no such command".

## Build and Validation

### Prerequisites
- **Rust:** 1.85+ (2024 edition) - check with `rustc --version`
- **OS:** Linux (some crates use Linux-specific APIs like io_uring)

### Build Order

Always run commands in this order for a clean validation:

```bash
# 1. Format check (fast, catches style issues early)
cargo fmt --all -- --check

# 2. Build (compiles all crates)
cargo build --workspace

# 3. Lint (catches warnings and issues)
cargo clippy --workspace -- -D warnings

# 4. Test (runs all unit tests)
cargo test --workspace
```

### CI Workflow Replication

The GitHub Actions CI (.github/workflows/ci.yml) runs these jobs:
1. **check** - `cargo check --workspace --all-features`
2. **test** - `cargo test --workspace --all-features`
3. **clippy** - `cargo clippy --workspace --all-features -- -D warnings`
4. **fmt** - `cargo fmt --all -- --check`
5. **docs** - `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS=-Dwarnings`
6. **msrv** - `cargo check --workspace` with Rust 1.85

To run the main CI checks locally, use: `cargo run -p xtask -- ci`

**Note:** The `xtask ci` command does **not** fully replicate the CI workflow above. It omits:
- The initial `cargo check --workspace --all-features` step
- Running clippy with `--all-features`
- Building docs with `RUSTDOCFLAGS=-Dwarnings`
- The MSRV (minimum supported Rust version) check

For a full CI replication, run these steps manually as described above.
## Repository Structure

```
WRAITH-Protocol/
├── Cargo.toml              # Workspace manifest (edition = "2024", rust-version = "1.85")
├── crates/
│   ├── wraith-core/        # Frame encoding, session state, congestion control
│   ├── wraith-crypto/      # AEAD, Elligator2, key ratcheting (Noise handshake stub)
│   ├── wraith-transport/   # UDP socket, io_uring stub
│   ├── wraith-obfuscation/ # Padding, timing modes
│   ├── wraith-discovery/   # DHT key derivation, relay stub
│   ├── wraith-files/       # File chunking, integrity verification
│   └── wraith-cli/         # CLI binary (wraith)
├── xtask/                  # Build automation (fmt, lint, test, ci, doc commands)
├── rustfmt.toml            # Formatter config (edition = "2021", max_width = 100)
├── clippy.toml             # Clippy config (msrv = "1.85")
└── .github/workflows/
    ├── ci.yml              # Main CI: check, test, clippy, fmt, docs, msrv
    └── codeql.yml          # Security scanning with CodeQL + cargo-audit
```

### Crate Dependencies
- **wraith-cli** depends on all other wraith crates
- **wraith-core** depends on wraith-crypto
- **wraith-transport/files/obfuscation/discovery** depend on wraith-core

### Key Configuration Files
- `Cargo.toml` - Workspace definition with shared dependencies (edition="2024")
- `rustfmt.toml` - Code formatting rules (edition="2021", max_width=100)
- `clippy.toml` - Linter settings (msrv="1.85")
- `.gitignore` - Excludes /target/, Cargo.lock, coverage files

**Note:** The rustfmt.toml uses edition="2021" for formatting rules while Cargo.toml uses edition="2024" for compilation. This is intentional as rustfmt edition controls formatting style.

## Making Changes

### Adding Dependencies
Add to `[workspace.dependencies]` in root Cargo.toml, then reference with `{ workspace = true }` in crate Cargo.toml files.

### Running Tests for Specific Crates
```bash
cargo test -p wraith-core
cargo test -p wraith-crypto
```

### Code Style
- Follow Rust conventions enforced by rustfmt and clippy
- Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`
- Document public APIs with doc comments
- No `unsafe` code in crypto paths without justification

### Known TODOs
Many modules contain TODO comments indicating incomplete implementation:
- `wraith-crypto/src/noise.rs` - Noise handshake needs implementation
- `wraith-crypto/src/elligator.rs` - Elligator2 encoding needs proper implementation
- `wraith-transport/src/udp.rs` and `io_uring_impl.rs` - Transport stubs
- `wraith-cli/src/main.rs` - CLI commands are stubs

## Validation Checklist

Before submitting changes, ensure:

1. **Format check passes:** `cargo fmt --all -- --check`
2. **Build succeeds:** `cargo build --workspace`
3. **Clippy passes:** `cargo clippy --workspace -- -D warnings`
4. **Tests pass:** `cargo test --workspace`
5. **Documentation builds:** `cargo doc --workspace --no-deps`

Or run most checks at once: `cargo run -p xtask -- ci`
_(Note: This does **not** run documentation build (`cargo doc --workspace --no-deps`) and may not use `--all-features` for clippy. Run those separately as needed.)_

## Common Issues

### "no such command: xtask"
**Cause:** Using `cargo xtask` instead of proper invocation  
**Solution:** Use `cargo run -p xtask -- <command>`

### Clippy warnings treated as errors
**Cause:** CI uses `-D warnings` flag  
**Solution:** Fix all clippy warnings before committing

### Build fails on non-Linux
**Cause:** io_uring crate is Linux-only  
**Solution:** Primary development should be on Linux

## PR Requirements

See `.github/PULL_REQUEST_TEMPLATE.md`:
- Code compiles without warnings
- All tests pass
- Clippy passes with `-D warnings`
- Code is formatted with `cargo fmt`
- CHANGELOG.md updated if applicable
- Conventional commit messages

## Trust These Instructions

These instructions were validated against the actual repository. If a command fails or behaves unexpectedly, first verify the command matches what's documented here. Only perform additional exploration if the documented commands are incomplete or incorrect.
