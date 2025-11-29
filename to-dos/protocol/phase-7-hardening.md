# Phase 7: Hardening & Optimization Sprint Planning

**Duration:** Weeks 37-44 (6-8 weeks)
**Total Story Points:** 145
**Risk Level:** Medium (security critical, time-consuming)

---

## Phase Overview

**Goal:** Prepare WRAITH Protocol for production deployment through comprehensive security audits, fuzzing, performance optimization, documentation, and cross-platform testing. Ensure the protocol is robust, secure, and ready for public release.

### Success Criteria

- [ ] No critical security issues
- [ ] Fuzz testing: 72 hours without crashes
- [ ] Performance targets met (10 Gbps, <1μs latency)
- [ ] Memory usage predictable and bounded
- [ ] Cross-platform builds succeed (Linux, macOS)
- [ ] Documentation complete (user guide, API docs, deployment guide)
- [ ] Deployment packages ready (deb, rpm, cargo)

### Dependencies

- Phase 6 complete (all components integrated)
- Access to security auditing tools
- Fuzzing infrastructure
- Profiling tools (perf, valgrind, flamegraph)

### Deliverables

1. Security audit report
2. Fuzzing harness for all parsers
3. Property-based tests for critical components
4. Performance optimizations
5. Memory leak detection and fixes
6. User documentation
7. API documentation (rustdoc)
8. Deployment guide
9. Monitoring/metrics system
10. Error recovery testing
11. Cross-platform CI
12. Release packages

---

## Sprint Breakdown

### Sprint 7.1: Security Audit (Weeks 37-39)

**Duration:** 3 weeks
**Story Points:** 34

**7.1.1: Cryptographic Code Review** (13 SP)

```bash
#!/bin/bash
# Security audit checklist

echo "=== WRAITH Protocol Security Audit ==="

# 1. Constant-time operations verification
echo "1. Verifying constant-time cryptographic operations..."
cargo test --features=crypto-ct-verify

# 2. Memory zeroization
echo "2. Checking memory zeroization..."
cargo clippy -- -W clippy::missing_zeroize

# 3. Integer overflow checks
echo "3. Checking for integer overflows..."
cargo build --release

# 4. Unsafe code audit
echo "4. Auditing unsafe code blocks..."
rg "unsafe" --type rust -A 5 | tee audit_unsafe.txt

# 5. Dependency audit
echo "5. Auditing dependencies..."
cargo audit

# 6. Secret detection
echo "6. Scanning for hardcoded secrets..."
gitleaks detect --source . --verbose

# 7. SAST (Static Application Security Testing)
echo "7. Running static analysis..."
cargo semver-checks check-release

echo "Audit complete. Review reports in audit_*.txt files."
```

**Manual Review Checklist:**

```markdown
## Cryptographic Implementation Review

### Key Management
- [ ] Private keys zeroized on drop
- [ ] No key material in logs
- [ ] No key material in error messages
- [ ] Secure random generation (OsRng)
- [ ] Key derivation uses approved KDFs

### Constant-Time Operations
- [ ] All crypto operations constant-time
- [ ] No secret-dependent branching
- [ ] No secret-dependent memory access
- [ ] Timing tests pass
- [ ] Side-channel resistance verified

### Encryption
- [ ] Nonces never reused
- [ ] Authentication tags verified
- [ ] AEAD used correctly
- [ ] No unauthenticated encryption
- [ ] Replay protection implemented

### Key Exchange
- [ ] DH parameters validated
- [ ] Weak public keys rejected
- [ ] Forward secrecy verified
- [ ] Ratcheting correct
- [ ] Session IDs unique

### Randomness
- [ ] All random from CSPRNGs
- [ ] No weak PRNGs (rand::thread_rng misuse)
- [ ] Sufficient entropy
- [ ] No predictable values

## Network Security Review

### Input Validation
- [ ] All packet parsing bounds-checked
- [ ] No buffer overflows possible
- [ ] Length fields validated
- [ ] No integer overflows in size calculations
- [ ] Malformed packets handled gracefully

### Denial of Service
- [ ] Rate limiting implemented
- [ ] Resource exhaustion prevented
- [ ] Amplification attacks mitigated
- [ ] Connection limits enforced
- [ ] Memory limits enforced

### Protocol Security
- [ ] No information leakage in errors
- [ ] No metadata leakage
- [ ] Timing attacks prevented
- [ ] Traffic analysis resistance verified
- [ ] DPI evasion tested

## Memory Safety Review

### Unsafe Code
- [ ] All unsafe blocks documented
- [ ] Safety invariants documented
- [ ] Bounds checking manual
- [ ] Pointer validity verified
- [ ] No use-after-free possible

### Resource Management
- [ ] No memory leaks
- [ ] File descriptors closed
- [ ] Sockets cleaned up
- [ ] Threads joined
- [ ] RAII used everywhere

## Concurrency Review

### Race Conditions
- [ ] Shared state properly synchronized
- [ ] No data races
- [ ] Lock ordering consistent
- [ ] Deadlock-free
- [ ] Atomic operations correct

### Thread Safety
- [ ] Send/Sync properly implemented
- [ ] No unsafe Send/Sync impls
- [ ] Channel usage correct
- [ ] No Arc<Mutex> misuse
```

**Acceptance Criteria:**
- [ ] All checklist items reviewed
- [ ] Critical issues fixed
- [ ] Audit report generated
- [ ] No known vulnerabilities
- [ ] Third-party review (if budget allows)

---

**7.1.2: Penetration Testing** (13 SP)

```python
#!/usr/bin/env python3
# Penetration testing scenarios for WRAITH Protocol

import socket
import struct
import random

def test_malformed_packets():
    """Send malformed packets to test parser robustness"""
    print("Testing malformed packet handling...")

    test_cases = [
        b"\x00" * 100,  # All zeros
        b"\xff" * 100,  # All ones
        b"",  # Empty packet
        random.randbytes(1000),  # Random data
        struct.pack("<I", 0xffffffff) + b"overflow",  # Max length field
    ]

    for i, packet in enumerate(test_cases):
        try:
            send_packet("127.0.0.1", 40000, packet)
            print(f"  Test {i+1}: Sent malformed packet")
        except Exception as e:
            print(f"  Test {i+1}: Exception (expected): {e}")

def test_replay_attacks():
    """Test replay attack prevention"""
    print("Testing replay attack prevention...")

    # Capture a legitimate packet
    legitimate_packet = capture_packet()

    # Replay it multiple times
    for i in range(10):
        send_packet("127.0.0.1", 40000, legitimate_packet)

    print("  Replay attack test complete")

def test_amplification():
    """Test for amplification vulnerabilities"""
    print("Testing amplification attacks...")

    small_request = b"\x01\x00\x00\x00QUERY"
    response_size = send_and_measure_response("127.0.0.1", 40000, small_request)

    amplification_factor = response_size / len(small_request)
    print(f"  Amplification factor: {amplification_factor:.2f}x")

    if amplification_factor > 10:
        print("  WARNING: High amplification factor!")

def test_resource_exhaustion():
    """Test resource exhaustion (DoS)"""
    print("Testing resource exhaustion...")

    # Connection exhaustion
    connections = []
    for i in range(10000):
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.connect(("127.0.0.1", 40000))
            connections.append(sock)
        except Exception as e:
            print(f"  Connection {i} failed: {e}")
            break

    print(f"  Created {len(connections)} connections")

    # Cleanup
    for sock in connections:
        sock.close()

def send_packet(host, port, data):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.sendto(data, (host, port))
    sock.close()

def capture_packet():
    # Simplified - would use scapy or similar in practice
    return b"CAPTURED_PACKET"

def send_and_measure_response(host, port, data):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(1.0)
    sock.sendto(data, (host, port))

    try:
        response, _ = sock.recvfrom(65536)
        return len(response)
    except socket.timeout:
        return 0
    finally:
        sock.close()

if __name__ == "__main__":
    print("=== WRAITH Protocol Penetration Testing ===\n")

    test_malformed_packets()
    test_replay_attacks()
    test_amplification()
    test_resource_exhaustion()

    print("\nPenetration testing complete.")
```

**Acceptance Criteria:**
- [ ] Malformed packets rejected gracefully
- [ ] Replay attacks detected
- [ ] No amplification vulnerabilities
- [ ] Resource limits enforced
- [ ] DoS resistance verified

---

**7.1.3: Dependency Security Audit** (8 SP)

```bash
#!/bin/bash
# Audit all dependencies for known vulnerabilities

echo "=== Dependency Security Audit ==="

# 1. Check for known vulnerabilities
echo "1. Checking for known CVEs..."
cargo audit

# 2. Check for outdated dependencies
echo "2. Checking for outdated dependencies..."
cargo outdated

# 3. Generate dependency tree
echo "3. Generating dependency tree..."
cargo tree > dependency_tree.txt

# 4. License compliance
echo "4. Checking license compliance..."
cargo license

# 5. Supply chain security
echo "5. Verifying crate checksums..."
cargo fetch --locked

# 6. Check for unmaintained crates
echo "6. Checking for unmaintained crates..."
cargo geiger --all-features

echo "Dependency audit complete."
```

**Acceptance Criteria:**
- [ ] No high/critical CVEs in dependencies
- [ ] All dependencies actively maintained
- [ ] License compatibility verified
- [ ] Supply chain verified (checksums)
- [ ] Minimal unsafe code in dependencies

---

### Sprint 7.2: Fuzzing & Property Testing (Weeks 39-40)

**Duration:** 2 weeks
**Story Points:** 26

**7.2.1: Fuzzing Harness** (13 SP)

```rust
// fuzz/fuzz_targets/frame_parser.rs

#![no_main]

use libfuzzer_sys::fuzz_target;
use wraith_core::frames::Frame;

fuzz_target!(|data: &[u8]| {
    // Fuzz frame parser
    let _ = Frame::parse(data);
});
```

```rust
// fuzz/fuzz_targets/dht_message.rs

#![no_main]

use libfuzzer_sys::fuzz_target;
use wraith_discovery::dht::DhtMessage;

fuzz_target!(|data: &[u8]| {
    // Fuzz DHT message parser
    let _ = DhtMessage::from_bytes(data);
});
```

```rust
// fuzz/fuzz_targets/crypto.rs

#![no_main]

use libfuzzer_sys::fuzz_target;
use wraith_crypto::noise::NoiseState;
use wraith_crypto::x25519::PrivateKey;

fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    // Fuzz Noise handshake
    let private_key = PrivateKey::from_bytes([0u8; 32]);
    let mut noise = NoiseState::new_xx(
        wraith_crypto::noise::Role::Responder,
        private_key,
        b"fuzz"
    );

    let _ = noise.read_message_1(data);
});
```

**Continuous Fuzzing:**

```yaml
# .github/workflows/fuzz.yml
name: Continuous Fuzzing

on:
  schedule:
    - cron: '0 0 * * *'  # Daily
  workflow_dispatch:

jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz

      - name: Run fuzzing (24 hours)
        run: |
          cargo fuzz run frame_parser -- -max_total_time=86400
          cargo fuzz run dht_message -- -max_total_time=86400
          cargo fuzz run crypto -- -max_total_time=86400

      - name: Upload crash artifacts
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: fuzz-crashes
          path: fuzz/artifacts/
```

**Acceptance Criteria:**
- [ ] Fuzzing harness for all parsers
- [ ] 72 hours continuous fuzzing without crashes
- [ ] Crashes fixed if found
- [ ] Coverage measurement (>80% code coverage)
- [ ] Fuzzing integrated in CI

---

**7.2.2: Property-Based Testing** (13 SP)

```rust
// tests/property_tests.rs

use proptest::prelude::*;
use wraith_core::frames::Frame;

proptest! {
    #[test]
    fn frame_encode_decode_roundtrip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
        let frame = Frame::Data {
            stream_id: 1,
            offset: 0,
            data: data.clone(),
            fin: false,
        };

        let encoded = frame.encode().unwrap();
        let decoded = Frame::parse(&encoded).unwrap();

        match decoded {
            Frame::Data { data: decoded_data, .. } => {
                assert_eq!(data, decoded_data);
            }
            _ => panic!("Wrong frame type"),
        }
    }

    #[test]
    fn x25519_key_exchange_symmetric(
        alice_secret in prop::array::uniform32(any::<u8>()),
        bob_secret in prop::array::uniform32(any::<u8>())
    ) {
        use wraith_crypto::x25519::PrivateKey;

        let alice_private = PrivateKey::from_bytes(alice_secret);
        let alice_public = alice_private.public_key();

        let bob_private = PrivateKey::from_bytes(bob_secret);
        let bob_public = bob_private.public_key();

        let alice_shared = alice_private.exchange(&bob_public).unwrap();
        let bob_shared = bob_private.exchange(&alice_public).unwrap();

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn padding_preserves_data(
        data in prop::collection::vec(any::<u8>(), 1..10000)
    ) {
        use wraith_obfuscation::padding::{PaddingEngine, PaddingMode};

        let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let original_len = data.len();

        let mut padded = data.clone();
        let target_size = engine.padded_size(original_len);
        engine.pad(&mut padded, target_size);

        let unpadded = engine.unpad(&padded, original_len);

        assert_eq!(unpadded, &data[..]);
    }

    #[test]
    fn dht_distance_metric_properties(
        id1 in prop::array::uniform32(any::<u8>()),
        id2 in prop::array::uniform32(any::<u8>()),
        id3 in prop::array::uniform32(any::<u8>())
    ) {
        use wraith_discovery::dht::NodeId;

        let node1 = NodeId(id1);
        let node2 = NodeId(id2);
        let node3 = NodeId(id3);

        // Distance to self is zero
        assert_eq!(node1.distance(&node1).0, [0u8; 32]);

        // Symmetric property
        assert_eq!(node1.distance(&node2).0, node2.distance(&node1).0);

        // Triangle inequality (should hold for XOR metric)
        // d(a, c) <= d(a, b) + d(b, c)
        let d_ac = node1.distance(&node3);
        let d_ab = node1.distance(&node2);
        let d_bc = node2.distance(&node3);

        // XOR metric satisfies triangle inequality
        // (This is a simplified check)
    }

    #[test]
    fn file_chunking_coverage(
        file_size in 1u64..10_000_000,
        chunk_size in 1024usize..1_048_576
    ) {
        let num_chunks = (file_size + chunk_size as u64 - 1) / chunk_size as u64;

        // Verify all bytes covered by chunks
        let mut covered_bytes = 0u64;

        for chunk_idx in 0..num_chunks {
            let chunk_offset = chunk_idx * chunk_size as u64;
            let chunk_len = (file_size - chunk_offset).min(chunk_size as u64);

            covered_bytes += chunk_len;
        }

        assert_eq!(covered_bytes, file_size);
    }
}
```

**Acceptance Criteria:**
- [ ] Property tests for all critical components
- [ ] Roundtrip properties verified
- [ ] Algebraic properties verified
- [ ] Edge cases covered
- [ ] Tests pass 10,000+ iterations

---

### Sprint 7.3: Performance Optimization (Weeks 40-42)

**Duration:** 2 weeks
**Story Points:** 34

**7.3.1: Profiling & Hotspot Identification** (13 SP)

```bash
#!/bin/bash
# Performance profiling script

echo "=== WRAITH Protocol Performance Profiling ==="

# 1. CPU profiling with perf
echo "1. Running CPU profiling..."
cargo build --release
perf record -F 99 -g -- ./target/release/wraith-bench
perf script | stackcollapse-perf.pl | flamegraph.pl > flamegraph.svg

echo "   Flamegraph generated: flamegraph.svg"

# 2. Memory profiling
echo "2. Running memory profiling..."
valgrind --tool=massif --massif-out-file=massif.out ./target/release/wraith-bench
ms_print massif.out > memory_profile.txt

echo "   Memory profile: memory_profile.txt"

# 3. Cache profiling
echo "3. Running cache profiling..."
perf stat -e cache-references,cache-misses,instructions,cycles \
    ./target/release/wraith-bench

# 4. Benchmarking
echo "4. Running benchmarks..."
cargo bench --all > benchmark_results.txt

echo "Profiling complete."
```

**Optimization Targets:**

```rust
// Identified hotspots and optimizations

// HOTSPOT 1: Frame parsing (15% CPU time)
// Optimization: SIMD for bounds checking, inline critical functions

#[inline(always)]
fn parse_frame_header(data: &[u8]) -> Option<FrameHeader> {
    if data.len() < FRAME_HEADER_SIZE {
        return None;
    }

    // Use SIMD for bounds checking and parsing
    // ...
}

// HOTSPOT 2: AEAD encryption (25% CPU time)
// Optimization: Use hardware AES-NI if available

#[cfg(target_feature = "aes")]
fn encrypt_chunk_aesni(chunk: &[u8], key: &[u8; 32]) -> Vec<u8> {
    // Hardware-accelerated encryption
    // ...
}

// HOTSPOT 3: Memory allocation in packet processing (10% CPU time)
// Optimization: Object pool for packet buffers

struct PacketPool {
    pool: Vec<Vec<u8>>,
}

impl PacketPool {
    fn acquire(&mut self) -> Vec<u8> {
        self.pool.pop().unwrap_or_else(|| vec![0u8; MAX_PACKET_SIZE])
    }

    fn release(&mut self, mut buf: Vec<u8>) {
        buf.clear();
        if self.pool.len() < MAX_POOL_SIZE {
            self.pool.push(buf);
        }
    }
}

// HOTSPOT 4: DHT lookups (8% CPU time)
// Optimization: Cache recent lookups

use lru::LruCache;

struct DhtCache {
    cache: LruCache<NodeId, Vec<DhtPeer>>,
}

impl DhtCache {
    fn lookup(&mut self, key: &NodeId) -> Option<&Vec<DhtPeer>> {
        self.cache.get(key)
    }

    fn insert(&mut self, key: NodeId, peers: Vec<DhtPeer>) {
        self.cache.put(key, peers);
    }
}
```

**Acceptance Criteria:**
- [ ] Profiling identifies all hotspots >5% CPU
- [ ] Top 3 hotspots optimized
- [ ] Performance improvement measured
- [ ] No regressions introduced
- [ ] Flamegraph shows balanced CPU usage

---

**7.3.2: Memory Optimization** (13 SP)

```rust
// Memory optimization techniques

// 1. Use stack allocation where possible
#[inline]
fn process_small_buffer(data: &[u8]) -> [u8; 256] {
    let mut result = [0u8; 256];
    // Process on stack
    result
}

// 2. Avoid unnecessary cloning
fn process_frame(frame: &Frame) {
    // Borrow instead of clone
    match frame {
        Frame::Data { data, .. } => {
            // Process without cloning data
        }
    }
}

// 3. Use Cow for conditional cloning
use std::borrow::Cow;

fn maybe_pad(data: &[u8], target_size: usize) -> Cow<[u8]> {
    if data.len() >= target_size {
        Cow::Borrowed(data)
    } else {
        let mut padded = data.to_vec();
        padded.resize(target_size, 0);
        Cow::Owned(padded)
    }
}

// 4. Use SmallVec for small allocations
use smallvec::SmallVec;

type SmallBuffer = SmallVec<[u8; 256]>;

fn allocate_buffer(size: usize) -> SmallBuffer {
    SmallVec::with_capacity(size)
}

// 5. Manual drop for large structures
struct LargeState {
    data: Vec<u8>,
}

impl Drop for LargeState {
    fn drop(&mut self) {
        // Explicitly clear sensitive data
        self.data.zeroize();
    }
}
```

**Memory Leak Detection:**

```bash
#!/bin/bash
# Memory leak detection

echo "=== Memory Leak Detection ==="

# 1. Valgrind memcheck
echo "1. Running Valgrind memcheck..."
valgrind --leak-check=full --show-leak-kinds=all \
    ./target/debug/wraith-daemon &

# Let it run for a while
sleep 300

killall wraith-daemon

# 2. AddressSanitizer
echo "2. Running with AddressSanitizer..."
RUSTFLAGS="-Z sanitizer=address" cargo build --target x86_64-unknown-linux-gnu
./target/x86_64-unknown-linux-gnu/debug/wraith-daemon &

sleep 300

killall wraith-daemon

echo "Leak detection complete. Review reports."
```

**Acceptance Criteria:**
- [ ] No memory leaks detected
- [ ] Memory usage bounded (per-session <10 MB)
- [ ] Pool allocators for hot paths
- [ ] Stack allocation optimized
- [ ] Zero-copy paths verified

---

**7.3.3: Algorithmic Optimizations** (8 SP)

```rust
// Algorithmic improvements

// 1. Use binary search for sorted lookups
fn find_chunk_index(chunks: &[ChunkInfo], offset: u64) -> Option<usize> {
    chunks.binary_search_by_key(&offset, |c| c.offset).ok()
}

// 2. Use HashSet for membership tests
use std::collections::HashSet;

fn is_chunk_received(chunk_index: u64, received: &HashSet<u64>) -> bool {
    received.contains(&chunk_index)
}

// 3. Batch operations
fn mark_chunks_received(session: &mut TransferSession, chunks: &[u64]) {
    for &chunk in chunks {
        session.transferred_chunks.insert(chunk);
    }
    // Update progress once after batch
    session.update_progress();
}

// 4. Use iterators instead of collect
fn process_missing_chunks(session: &TransferSession) -> impl Iterator<Item = u64> + '_ {
    (0..session.total_chunks)
        .filter(move |i| !session.transferred_chunks.contains(i))
}

// 5. Lazy evaluation
use once_cell::sync::Lazy;

static GLOBAL_CONFIG: Lazy<Config> = Lazy::new(|| {
    Config::load().unwrap_or_default()
});
```

**Acceptance Criteria:**
- [ ] O(n) algorithms replaced with O(log n) where possible
- [ ] Unnecessary allocations eliminated
- [ ] Iterator chains optimized
- [ ] Lazy initialization for expensive operations
- [ ] Benchmark improvements documented

---

### Sprint 7.4: Documentation (Weeks 42-43)

**Duration:** 1.5 weeks
**Story Points:** 26

**7.4.1: User Documentation** (13 SP)

```markdown
# WRAITH Protocol User Guide

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Configuration](#configuration)
4. [Usage](#usage)
5. [Troubleshooting](#troubleshooting)
6. [FAQ](#faq)

## Installation

### Linux (Ubuntu/Debian)

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install clang llvm libbpf-dev

# Install WRAITH
wget https://github.com/wraith-protocol/wraith/releases/latest/download/wraith_amd64.deb
sudo dpkg -i wraith_amd64.deb
```

### Linux (Fedora/RHEL)

```bash
# Install dependencies
sudo dnf install clang llvm libbpf-devel

# Install WRAITH
sudo rpm -i https://github.com/wraith-protocol/wraith/releases/latest/download/wraith_x86_64.rpm
```

### macOS

```bash
brew install wraith
```

### From Source

```bash
git clone https://github.com/wraith-protocol/wraith
cd wraith
cargo install --path wraith-cli
```

## Quick Start

### Send a file

```bash
# Start receiver
wraith receive --output ~/Downloads

# In another terminal, send file
wraith send document.pdf --to <peer-id>
```

### Generate node identity

```bash
wraith daemon --init
# Creates ~/.wraith/config.toml with new keypair
```

## Configuration

Default configuration file: `~/.wraith/config.toml`

```toml
[node]
public_key = "..."  # Auto-generated
private_key_file = "~/.wraith/private_key"

[network]
listen_addr = "0.0.0.0:40000"
enable_xdp = false  # Requires root and AF_XDP support

[obfuscation]
default_level = "medium"  # none, low, medium, high, paranoid

[discovery]
bootstrap_nodes = [
    "bootstrap1.wraith.network:40000",
]
```

## Usage

### File Transfer

```bash
# Send file
wraith send file.zip --to <peer-id>

# Receive (daemon mode)
wraith daemon

# In another terminal:
wraith transfers  # List active transfers
```

### Advanced Options

```bash
# High obfuscation
wraith send file.zip --to <peer-id> --obfuscation paranoid

# Resume interrupted transfer
wraith send file.zip --to <peer-id> --resume

# Multi-peer download
wraith receive --multi-peer
```

## Troubleshooting

### Connection fails

```bash
# Check NAT type
wraith status --nat-type

# Test relay connectivity
wraith test-relay relay1.wraith.network:40001
```

### Low throughput

```bash
# Enable XDP (requires root)
sudo wraith daemon --xdp --interface eth0

# Check congestion control
wraith status --congestion
```

## FAQ

**Q: Is WRAITH anonymous?**
A: WRAITH provides traffic obfuscation and encryption, but not anonymity like Tor. Peers know each other's IP addresses.

**Q: Can I run WRAITH on Windows?**
A: Limited support (UDP fallback only, no XDP).

**Q: How do I find peer IDs?**
A: Use DHT discovery or exchange peer IDs out-of-band.
```

**Acceptance Criteria:**
- [ ] Installation guide for all platforms
- [ ] Quick start guide
- [ ] Configuration reference
- [ ] Usage examples
- [ ] Troubleshooting guide
- [ ] FAQ

---

**7.4.2: API Documentation** (8 SP)

```rust
//! # WRAITH Protocol Library
//!
//! WRAITH is a high-performance, secure, peer-to-peer file transfer protocol.
//!
//! ## Features
//!
//! - **Security**: XChaCha20-Poly1305 encryption, Noise_XX handshake, forward secrecy
//! - **Performance**: 10+ Gbps throughput with AF_XDP kernel bypass
//! - **Stealth**: Traffic obfuscation, protocol mimicry, DPI evasion
//! - **P2P**: DHT peer discovery, NAT traversal, relay fallback
//!
//! ## Quick Example
//!
//! ```rust,no_run
//! use wraith_core::Node;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create node
//!     let node = Node::new_random().await?;
//!
//!     // Send file
//!     let transfer_id = node.send_file(
//!         "document.pdf",
//!         &peer_public_key
//!     ).await?;
//!
//!     // Wait for completion
//!     node.wait_for_transfer(transfer_id).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! WRAITH consists of multiple crates:
//!
//! - `wraith-core`: Core protocol types and session management
//! - `wraith-crypto`: Cryptographic primitives (Noise, AEAD, hashing)
//! - `wraith-transport`: Network transport (XDP, UDP, io_uring)
//! - `wraith-obfuscation`: Traffic obfuscation (padding, mimicry)
//! - `wraith-discovery`: Peer discovery (DHT, relay, NAT traversal)
//! - `wraith-files`: File chunking and tree hashing
//! - `wraith-cli`: Command-line interface
//!
//! ## Security Considerations
//!
//! - Always verify `tree_hash` before accepting transfers
//! - Use `obfuscation_level = high` in adversarial networks
//! - Rotate keys periodically
//! - Keep relay servers updated
//!
//! ## Performance Tuning
//!
//! For maximum performance:
//!
//! ```toml
//! [network]
//! enable_xdp = true  # Requires root
//! xdp_interface = "eth0"
//!
//! [transfer]
//! chunk_size = 262144  # 256 KB
//! ```

/// WRAITH protocol node
///
/// # Examples
///
/// ```rust,no_run
/// use wraith_core::Node;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let node = Node::new_random().await?;
/// # Ok(())
/// # }
/// ```
pub struct Node {
    // ...
}

impl Node {
    /// Create a new node with random keypair
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use wraith_core::Node;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let node = Node::new_random().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new_random() -> Result<Self, Error> {
        // ...
    }

    /// Send file to peer
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to file to send
    /// * `peer_public_key` - Recipient's public key
    ///
    /// # Returns
    ///
    /// Transfer ID for tracking progress
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use wraith_core::Node;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let node = Node::new_random().await?;
    /// # let peer_key = [0u8; 32];
    /// let transfer_id = node.send_file("file.zip", &peer_key).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_file(
        &self,
        file_path: impl AsRef<Path>,
        peer_public_key: &[u8; 32],
    ) -> Result<TransferId, Error> {
        // ...
    }
}
```

**Acceptance Criteria:**
- [ ] All public APIs documented
- [ ] Examples for common use cases
- [ ] Security notes where relevant
- [ ] Performance notes
- [ ] `cargo doc` generates clean documentation
- [ ] Doctests pass

---

**7.4.3: Deployment Guide** (5 SP)

```markdown
# WRAITH Protocol Deployment Guide

## Production Deployment

### System Requirements

**Minimum:**
- Linux kernel 5.10+
- 2 CPU cores
- 4 GB RAM
- 100 Mbps network

**Recommended:**
- Linux kernel 6.6+ (AF_XDP support)
- 8+ CPU cores
- 16 GB RAM
- 10 Gbps NIC (Intel X710 or Mellanox ConnectX-5)
- NVMe SSD

### Installation

```bash
# Install WRAITH
sudo dpkg -i wraith_amd64.deb

# Configure systemd service
sudo systemctl enable wraith
sudo systemctl start wraith

# Verify
sudo systemctl status wraith
```

### Configuration

Production config (`/etc/wraith/config.toml`):

```toml
[node]
private_key_file = "/etc/wraith/private_key"

[network]
listen_addr = "0.0.0.0:40000"
enable_xdp = true
xdp_interface = "eth0"

[obfuscation]
default_level = "high"
tls_mimicry = true

[discovery]
bootstrap_nodes = [
    "bootstrap1.wraith.network:40000",
    "bootstrap2.wraith.network:40000",
]

[logging]
level = "info"
file = "/var/log/wraith/wraith.log"
```

### Security Hardening

```bash
# Firewall rules
sudo ufw allow 40000/udp
sudo ufw allow 40000/tcp

# Run as non-root (if not using XDP)
sudo -u wraith wraith daemon

# Enable AppArmor profile
sudo aa-enforce /etc/apparmor.d/usr.bin.wraith
```

### Monitoring

```bash
# Check status
wraith status

# Monitor logs
tail -f /var/log/wraith/wraith.log

# Metrics endpoint
curl http://localhost:9090/metrics
```

## Relay Server Deployment

```bash
# Install
sudo apt-get install wraith-relay

# Configure
sudo nano /etc/wraith-relay/config.toml

# Start
sudo systemctl start wraith-relay
```

## Bootstrap Node Deployment

```bash
# Use Docker
docker run -d \
    --name wraith-bootstrap \
    -p 40000:40000/udp \
    -v /etc/wraith:/etc/wraith \
    wraith/bootstrap:latest
```

## Performance Tuning

```bash
# Enable huge pages
echo 1024 | sudo tee /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages

# Tune network stack
sudo sysctl -w net.core.rmem_max=134217728
sudo sysctl -w net.core.wmem_max=134217728

# CPU governor
sudo cpupower frequency-set -g performance
```

## Backup & Recovery

```bash
# Backup keys
sudo cp /etc/wraith/private_key /secure/backup/

# Backup config
sudo cp /etc/wraith/config.toml /secure/backup/

# Restore
sudo cp /secure/backup/private_key /etc/wraith/
sudo systemctl restart wraith
```

## Troubleshooting

### High CPU usage

```bash
# Check profiling
wraith profile --duration 60

# Disable XDP if necessary
sudo nano /etc/wraith/config.toml
# Set enable_xdp = false
sudo systemctl restart wraith
```

### Connection issues

```bash
# Test connectivity
wraith test-connection <peer-addr>

# Check NAT status
wraith nat-status

# Enable relay fallback
# (usually enabled by default)
```
```

**Acceptance Criteria:**
- [ ] Production deployment guide
- [ ] Security hardening checklist
- [ ] Monitoring setup
- [ ] Troubleshooting procedures
- [ ] Performance tuning guide

---

### Sprint 7.5: Cross-Platform & Packaging (Weeks 43-44)

**Duration:** 1.5 weeks
**Story Points:** 25

**7.5.1: Cross-Platform CI** (13 SP)

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  test-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install clang llvm libbpf-dev

      - name: Run tests
        run: cargo test --all --verbose

      - name: Run clippy
        run: cargo clippy --all -- -D warnings

      - name: Check formatting
        run: cargo fmt --all -- --check

  test-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Run tests (UDP fallback only)
        run: cargo test --all --features=udp-only

  build-packages:
    needs: [test-linux, test-macos]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build deb package
        run: cargo deb

      - name: Build rpm package
        run: cargo generate-rpm

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: packages
          path: |
            target/debian/*.deb
            target/generate-rpm/*.rpm
```

**Acceptance Criteria:**
- [ ] CI passes on Linux
- [ ] CI passes on macOS (UDP fallback)
- [ ] Builds succeed on both platforms
- [ ] Tests pass on both platforms
- [ ] Packages generated automatically

---

**7.5.2: Release Packages** (12 SP)

```toml
# Cargo.toml

[package.metadata.deb]
maintainer = "WRAITH Team <team@wraith.network>"
copyright = "2025, WRAITH Team"
license-file = ["LICENSE", "4"]
extended-description = """\
WRAITH Protocol - High-performance, secure, peer-to-peer file transfer protocol.
Features: XChaCha20-Poly1305 encryption, AF_XDP kernel bypass, DHT peer discovery."""
depends = "$auto, libbpf0"
section = "net"
priority = "optional"
assets = [
    ["target/release/wraith", "usr/bin/", "755"],
    ["config.toml", "etc/wraith/", "644"],
    ["README.md", "usr/share/doc/wraith/", "644"],
]

[package.metadata.generate-rpm]
assets = [
    { source = "target/release/wraith", dest = "/usr/bin/wraith", mode = "755" },
    { source = "config.toml", dest = "/etc/wraith/config.toml", mode = "644" },
]
```

**Release Workflow:**

```bash
#!/bin/bash
# Release script

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

echo "Creating WRAITH release v$VERSION"

# 1. Update version in Cargo.toml
sed -i "s/^version = .*/version = \"$VERSION\"/" Cargo.toml

# 2. Build release binaries
cargo build --release

# 3. Run tests
cargo test --all --release

# 4. Build packages
cargo deb
cargo generate-rpm

# 5. Create git tag
git tag -a "v$VERSION" -m "Release v$VERSION"
git push origin "v$VERSION"

# 6. Create GitHub release
gh release create "v$VERSION" \
    --title "WRAITH v$VERSION" \
    --notes "See CHANGELOG.md for details" \
    target/debian/*.deb \
    target/generate-rpm/*.rpm

echo "Release v$VERSION complete!"
```

**Acceptance Criteria:**
- [ ] Deb package builds correctly
- [ ] RPM package builds correctly
- [ ] Packages install cleanly
- [ ] Binary works after install
- [ ] Release automation works

---

## Definition of Done (Phase 7)

### Security
- [ ] Security audit complete
- [ ] No critical vulnerabilities
- [ ] Fuzzing: 72 hours without crashes
- [ ] Penetration testing passed
- [ ] Dependency audit clean

### Performance
- [ ] All targets met (10 Gbps, <1μs latency)
- [ ] Memory usage bounded
- [ ] No memory leaks
- [ ] Hotspots optimized
- [ ] Benchmarks documented

### Quality
- [ ] Code coverage >85%
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Documentation complete
- [ ] Examples working

### Deployment
- [ ] Cross-platform builds
- [ ] Packages generated
- [ ] Deployment guide complete
- [ ] Monitoring setup documented
- [ ] CI/CD functional

---

## Risk Mitigation

### Security Vulnerabilities
**Risk**: Critical vulnerabilities found late
**Mitigation**: Continuous fuzzing, early audits, security-focused code reviews

### Performance Regressions
**Risk**: Optimizations break functionality
**Mitigation**: Comprehensive benchmarks, regression tests

### Documentation Debt
**Risk**: Documentation incomplete or outdated
**Mitigation**: Docs in CI, review requirements for PRs

---

## Phase 7 Completion Checklist

- [ ] Sprint 7.1: Security audit
- [ ] Sprint 7.2: Fuzzing & property testing
- [ ] Sprint 7.3: Performance optimization
- [ ] Sprint 7.4: Documentation
- [ ] Sprint 7.5: Cross-platform & packaging
- [ ] All acceptance criteria met
- [ ] Production-ready
- [ ] Release v1.0 prepared

**Estimated Completion:** Week 44 (end of Phase 7)

---

**WRAITH Protocol v1.0 READY FOR RELEASE!**
