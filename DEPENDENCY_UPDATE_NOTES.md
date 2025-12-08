# Dependency Update Notes - rand_core 0.9

## Summary

This document explains why the Dependabot PR #21 to update `rand_core` from 0.6 to 0.9 cannot be applied at this time.

## Issue

Dependabot proposed updating `rand_core` from version 0.6 to 0.9. However, after investigation, **rand_core 0.9 is incompatible with the current stable Rust cryptographic ecosystem** used by WRAITH Protocol.

## Root Cause

The WRAITH Protocol depends on several cryptographic libraries from the Dalek cryptography ecosystem:

- `ed25519-dalek` 2.1/2.2 (for Ed25519 signatures)
- `x25519-dalek` 2.0 (for X25519 key exchange)
- `chacha20poly1305` 0.10 (for AEAD encryption, used by `snow` Noise protocol)

All of these stable versions depend on `rand_core` 0.6.x, creating a dependency conflict when trying to upgrade to `rand_core` 0.9.

### Dependency Chain

```
wraith-crypto
├── ed25519-dalek 2.2.0 → rand_core 0.6.4 (via crypto-common)
├── x25519-dalek 2.0.1 → rand_core 0.6.4 (via crypto-common)
└── chacha20poly1305 0.10.1 → crypto-common 0.1.7 → rand_core 0.6.4
```

## Attempted Solutions

### Option 1: Update dalek crates to 3.0 pre-release

The dalek cryptography team has 3.0.0-pre.3 pre-release versions that support newer `rand_core`:

- `ed25519-dalek` 3.0.0-pre.3
- `x25519-dalek` 3.0.0-pre.3

However, these versions use `rand_core` 0.10.0-rc-2, not 0.9.x, which creates a three-way version conflict:

```
wraith-crypto
├── ed25519-dalek 3.0.0-pre.3 → rand_core 0.10.0-rc-2
├── x25519-dalek 3.0.0-pre.3 → rand_core 0.10.0-rc-2
├── chacha20poly1305 0.10.1 → rand_core 0.6.4
└── rand_core 0.9.3 (from workspace)
```

This results in compile errors due to trait incompatibilities across three different `rand_core` versions.

### Option 2: Update chacha20poly1305

While `chacha20poly1305` 0.11.0-rc.2 exists, it would require updating `snow` (the Noise protocol implementation), which may have cascading effects on the protocol implementation.

## Recommendation

**Keep `rand_core` at version 0.6.x** until the Rust cryptographic ecosystem stabilizes with consistent `rand_core` versions.

Monitor for:
1. Stable releases of `ed25519-dalek` 3.x and `x25519-dalek` 3.x
2. Stable release of `rand_core` 0.10.x or later
3. Updates to `chacha20poly1305` and `snow` that support the new ecosystem

## Changes Applied

Based on code review feedback, the following changes were made:

1. **Reverted `rand_core` to 0.6**: Changed from 0.9 back to 0.6 for compatibility
2. **Updated `wraith-cli/Cargo.toml`**: Changed `rand_core = "0.6"` to `rand_core = { workspace = true }` for consistency with other workspace members
3. **Did not update `rand`**: Kept at 0.8 since it's compatible with `rand_core` 0.6

## Testing

All changes have been validated:
- ✅ `cargo build --workspace` succeeds
- ✅ `cargo test --workspace` passes
- ✅ `cargo clippy --workspace -- -D warnings` passes
- ✅ `cargo fmt --all -- --check` passes
- ✅ No security vulnerabilities found in dependencies

## Future Work

When updating to `rand_core` 0.9 or later becomes feasible, the following dependencies should be updated together:

- `rand_core`: 0.6 → 0.9+ (change feature from `getrandom` to `std` or `os_rng`)
- `rand`: 0.8 → 0.9+
- `ed25519-dalek`: 2.x → 3.x (stable)
- `x25519-dalek`: 2.x → 3.x (stable)
- `chacha20poly1305`: Verify compatibility
- `snow`: Verify compatibility

This should be done as a coordinated update in a single PR to avoid version conflicts.
