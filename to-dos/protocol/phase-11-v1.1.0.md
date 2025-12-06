# Phase 11: v1.1.0 - End-to-End Integration & Production Release

**Target:** v1.1.0 Production Release - Complete Protocol Integration + Hardening
**Estimated Effort:** 128 Story Points (~6-8 weeks)
**Prerequisites:** Phase 10 Sessions 2-4 complete - Components integrated, benchmarks validated

---

## Overview

Phase 11 completes the WRAITH Protocol implementation by addressing the critical infrastructure gap identified in Phase 10 Session 4, implementing all deferred production hardening features, and preparing for public release.

**Phase 10 → Phase 11 Context:**

Phase 10 Sessions 2-4 successfully delivered:
- ✅ Transport layer integration (UDP with async I/O)
- ✅ Noise_XX handshake implementation
- ✅ Discovery manager integration (DHT, NAT, relay)
- ✅ Crypto integration (frame encryption, key ratcheting)
- ✅ File transfer integration (chunking, tree hashing, reassembly)
- ✅ Obfuscation integration (padding, timing, protocol mimicry)
- ✅ File operations benchmarking (14.85 GiB/s chunking, 4.71 GiB/s hashing)
- ✅ 40 integration tests passing (1,025 total tests, 100% pass rate)

**Critical Infrastructure Gap Identified:**

Performance testing revealed **packet routing infrastructure** is missing, blocking:
- 7 deferred integration tests (marked `#[ignore]`)
- Network performance benchmarks (throughput, latency, BBR, multi-peer)
- End-to-end protocol validation

**What Phase 11 Delivers:**

1. **Packet Routing Layer** - Node-to-Node communication infrastructure (CRITICAL)
2. **Network Performance Validation** - Validate >300 Mbps throughput, <10ms latency
3. **Production Hardening** - Rate limiting, health monitoring, error recovery
4. **Advanced Features** - Resume robustness, migration stress testing, multi-peer optimization
5. **XDP Documentation** - Document why unavailable, fallback behavior
6. **Complete Documentation** - Tutorials, integration guides, troubleshooting
7. **Security Validation** - 72-hour fuzzing, DPI evasion testing
8. **Reference Client** - Minimal GUI demonstrating protocol usage

**v1.0.0 vs v1.1.0 Scope Adjustment:**

The original Phase 10 plan targeted v1.0.0 with 93 SP across 4 sprints. Phase 10 Sessions 2-4 completed component integration (~60% of original scope), but deferred several items due to packet routing dependency. Phase 11 v1.1.0 completes the remaining scope plus the packet routing infrastructure.

---

## Sprint 11.1: Packet Routing & End-to-End Integration (Weeks 1-2)

**Duration:** 2 weeks
**Story Points:** 34
**Goal:** Implement packet routing infrastructure to enable Node-to-Node communication and unblock deferred integration tests

### 11.1.1: Packet Routing Infrastructure (21 SP)

**Objective:** Build the missing packet routing layer that routes incoming packets to the correct session and enables background packet processing.

**Problem Statement:**

The Node API orchestration layer is complete, but lacks the infrastructure to route packets between nodes. Current implementation can:
- Bind UDP transport and receive packets
- Perform Noise_XX handshakes
- Encrypt/decrypt frames
- Manage sessions and transfers

But cannot:
- Route incoming packets to the correct session (by Connection ID)
- Process packets in background (spawned receiver loop exists but doesn't route)
- Enable actual node-to-node communication (loopback or network)

**Implementation:**

#### 1. Connection ID Routing Map

```rust
// crates/wraith-core/src/node/routing.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::node::{SessionId, PeerConnection};

/// Packet routing table: Connection ID → PeerConnection
pub struct RoutingTable {
    /// Map Connection ID (8 bytes) to session
    routes: Arc<RwLock<HashMap<u64, Arc<RwLock<PeerConnection>>>>>,
}

impl RoutingTable {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add route for new session
    pub async fn add_route(&self, connection_id: u64, connection: Arc<RwLock<PeerConnection>>) {
        self.routes.write().await.insert(connection_id, connection);
    }

    /// Remove route when session closes
    pub async fn remove_route(&self, connection_id: u64) {
        self.routes.write().await.remove(&connection_id);
    }

    /// Lookup session by Connection ID
    pub async fn lookup(&self, connection_id: u64) -> Option<Arc<RwLock<PeerConnection>>> {
        self.routes.read().await.get(&connection_id).cloned()
    }

    /// Get all active routes
    pub async fn active_routes(&self) -> Vec<u64> {
        self.routes.read().await.keys().copied().collect()
    }
}
```

#### 2. Enhanced Packet Receiver with Routing

```rust
// crates/wraith-core/src/node/node.rs

async fn packet_receive_loop(&self) {
    let mut buf = vec![0u8; 65536]; // 64 KB buffer for jumbo frames

    loop {
        if !self.inner.running.load(Ordering::SeqCst) {
            break;
        }

        // Get transport
        let transport = match self.inner.transport.lock().await.as_ref() {
            Some(t) => t.clone(),
            None => {
                tracing::warn!("Transport not initialized, stopping packet loop");
                break;
            }
        };

        // Receive packet with timeout
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            transport.recv_from(&mut buf)
        ).await;

        match result {
            Ok(Ok((size, from))) => {
                // Parse outer packet to extract Connection ID
                if size < 8 {
                    tracing::warn!("Packet too small (< 8 bytes), dropping");
                    continue;
                }

                // Extract Connection ID (first 8 bytes)
                let connection_id = u64::from_be_bytes(buf[0..8].try_into().unwrap());

                // Lookup session by Connection ID
                let connection = match self.inner.routing.lookup(connection_id).await {
                    Some(conn) => conn,
                    None => {
                        // Unknown Connection ID - might be handshake initiation
                        self.handle_new_connection(&buf[..size], from).await;
                        continue;
                    }
                };

                // Spawn handler for this packet (parallel processing)
                let node = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = node.handle_packet(connection, &buf[..size]).await {
                        tracing::warn!("Packet handling failed: {}", e);
                    }
                });
            }
            Ok(Err(e)) => {
                tracing::warn!("Transport receive error: {}", e);
            }
            Err(_) => {
                // Timeout - check running state and continue
            }
        }
    }
}

/// Handle packet for known session
async fn handle_packet(
    &self,
    connection: Arc<RwLock<PeerConnection>>,
    packet: &[u8],
) -> Result<(), NodeError> {
    // Parse encrypted frame
    let frame = {
        let conn = connection.read().await;
        conn.decrypt_frame(packet)?
    };

    // Route frame to appropriate handler
    match frame.frame_type() {
        FrameType::Data => self.handle_data_frame(connection, frame).await,
        FrameType::Ack => self.handle_ack_frame(connection, frame).await,
        FrameType::Control => self.handle_control_frame(connection, frame).await,
        FrameType::Ping => self.handle_ping_frame(connection, frame).await,
        FrameType::Close => self.handle_close_frame(connection, frame).await,
        _ => {
            tracing::warn!("Unknown frame type: {:?}", frame.frame_type());
            Ok(())
        }
    }
}

/// Handle new connection (handshake initiation)
async fn handle_new_connection(&self, packet: &[u8], from: SocketAddr) {
    tracing::debug!("Received potential handshake from {}", from);

    // Check if this is a Noise handshake message 1
    // (implementation deferred to Session 11.1.2)
}
```

#### 3. Update Session Establishment to Add Routes

```rust
impl Node {
    pub async fn establish_session(&self, peer_id: &PeerId) -> Result<SessionId, NodeError> {
        // ... existing handshake code ...

        // Create PeerConnection
        let connection = PeerConnection::new(session_id, *peer_id, peer_addr, connection_id, crypto);
        let connection = Arc::new(RwLock::new(connection));

        // Add to routing table
        self.inner.routing.add_route(connection_id, Arc::clone(&connection)).await;

        // Add to sessions map
        self.inner.sessions.write().await.insert(*peer_id, connection);

        Ok(session_id)
    }

    pub async fn close_session(&self, peer_id: &PeerId) -> Result<(), NodeError> {
        // Remove from sessions
        let connection = self.inner.sessions.write().await.remove(peer_id)
            .ok_or(NodeError::SessionNotFound)?;

        // Remove from routing table
        let connection_id = connection.read().await.connection_id();
        self.inner.routing.remove_route(connection_id).await;

        // Send CLOSE frame
        // ... existing close logic ...

        Ok(())
    }
}
```

#### 4. Node State Updates

```rust
pub(crate) struct NodeInner {
    // ... existing fields ...

    /// Packet routing table (Connection ID → PeerConnection)
    pub(crate) routing: RoutingTable,
}

impl Node {
    pub fn new_random() -> Result<Self, NodeError> {
        // ... existing initialization ...

        let inner = Arc::new(NodeInner {
            // ... existing fields ...
            routing: RoutingTable::new(),
        });

        Ok(Self { inner })
    }
}
```

**Tasks:**
- [ ] Implement `RoutingTable` with Connection ID → PeerConnection mapping
- [ ] Update `packet_receive_loop()` to extract Connection ID and route packets
- [ ] Implement `handle_packet()` to decrypt and dispatch frames
- [ ] Update `establish_session()` to add routes
- [ ] Update `close_session()` to remove routes
- [ ] Handle unknown Connection IDs (new handshakes)
- [ ] Add routing table statistics (active routes, lookups/sec)
- [ ] Write 8 tests:
  - Route addition/removal
  - Packet routing to correct session
  - Unknown Connection ID handling
  - Concurrent route lookups
  - Route cleanup on session close
  - Routing table statistics
  - Frame dispatching (DATA, ACK, CONTROL, etc.)
  - Background packet processing

**Acceptance Criteria:**
- [ ] Incoming packets routed to correct session by Connection ID
- [ ] Background packet processing loop functional
- [ ] Unknown Connection IDs handled gracefully
- [ ] Routing table scales to 1000+ sessions
- [ ] All 8 tests passing
- [ ] Zero packet drops due to routing errors

---

### 11.1.2: Un-Ignore Deferred Integration Tests (8 SP)

**Objective:** Enable and validate the 7 integration tests deferred in Phase 10 Session 4.

**Deferred Tests:**

1. **test_noise_handshake_loopback** - Noise_XX handshake between two nodes
2. **test_end_to_end_file_transfer** - Complete file transfer workflow
3. **test_connection_establishment** - Session establishment over network
4. **test_discovery_and_peer_finding** - DHT peer lookup
5. **test_multi_path_transfer_node_api** - Multi-peer download
6. **test_error_recovery_node_api** - Network error handling
7. **test_concurrent_transfers_node_api** - Multiple simultaneous transfers

**Implementation Steps:**

#### 1. Update Test Infrastructure

```rust
// tests/integration_tests.rs

/// Helper: Create two nodes and perform loopback handshake
async fn setup_two_node_loopback() -> (Node, Node) {
    // Node 1: Listen on port 0 (automatic)
    let config1 = NodeConfig {
        listen_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };
    let node1 = Node::new_with_config(config1).unwrap();
    node1.start().await.unwrap();

    // Get node1's actual listen address
    let addr1 = node1.listen_addr().await.unwrap();

    // Node 2: Listen on different port
    let config2 = NodeConfig {
        listen_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };
    let node2 = Node::new_with_config(config2).unwrap();
    node2.start().await.unwrap();

    (node1, node2)
}
```

#### 2. Enable Tests One-by-One

```rust
#[tokio::test]
async fn test_noise_handshake_loopback() {
    let (node1, node2) = setup_two_node_loopback().await;

    // Node1 establishes session with Node2
    let node2_id = node2.node_id();
    let node2_addr = node2.listen_addr().await.unwrap();

    let session_id = node1.establish_session_with_addr(&node2_id, node2_addr).await.unwrap();

    // Verify session exists
    assert!(node1.active_sessions().await.contains(&node2_id));

    // Verify session ID is valid (32 bytes)
    assert_eq!(session_id.len(), 32);

    // Clean up
    node1.stop().await.unwrap();
    node2.stop().await.unwrap();
}

#[tokio::test]
async fn test_end_to_end_file_transfer() {
    let (node1, node2) = setup_two_node_loopback().await;

    // Create test file
    let file_path = "/tmp/wraith-test-file-1mb.dat";
    let file_data = vec![0xAA; 1_000_000]; // 1 MB
    tokio::fs::write(file_path, &file_data).await.unwrap();

    // Node1 sends file to Node2
    let node2_id = node2.node_id();
    let node2_addr = node2.listen_addr().await.unwrap();

    let transfer_id = node1.send_file_to_addr(file_path, &node2_id, node2_addr).await.unwrap();

    // Wait for transfer completion (with timeout)
    tokio::time::timeout(
        Duration::from_secs(10),
        node1.wait_for_transfer(transfer_id)
    ).await.unwrap().unwrap();

    // Verify file received
    let received_path = format!("/tmp/wraith-received-{}.dat", transfer_id);
    let received_data = tokio::fs::read(&received_path).await.unwrap();
    assert_eq!(received_data, file_data);

    // Clean up
    tokio::fs::remove_file(file_path).await.unwrap();
    tokio::fs::remove_file(received_path).await.unwrap();
    node1.stop().await.unwrap();
    node2.stop().await.unwrap();
}

// Similar updates for remaining 5 tests...
```

**Tasks:**
- [ ] Remove `#[ignore]` from test_noise_handshake_loopback
- [ ] Add helper functions for two-node setup
- [ ] Verify handshake works over loopback
- [ ] Remove `#[ignore]` from test_end_to_end_file_transfer
- [ ] Implement file send/receive over loopback
- [ ] Remove `#[ignore]` from remaining 5 tests
- [ ] Update test infrastructure for multi-node scenarios
- [ ] Run all 7 tests in parallel (port conflicts avoided with port 0)
- [ ] Validate 100% pass rate

**Acceptance Criteria:**
- [ ] All 7 deferred tests passing
- [ ] Tests run reliably in CI (no flakiness)
- [ ] Port conflicts avoided (automatic port selection)
- [ ] Test coverage for all major workflows
- [ ] Integration test count: 47 total (47 passing, 0 ignored)

---

### 11.1.3: Enable Network Performance Benchmarks (5 SP)

**Objective:** Implement and run the 4 network benchmarks deferred in Phase 10 Session 4.

**Benchmarks to Implement:**

1. **Transfer Throughput** - Target: >300 Mbps on 1 Gbps LAN
2. **Transfer Latency** - Target: <10ms RTT on LAN
3. **BBR Utilization** - Target: >95% link utilization
4. **Multi-Peer Speedup** - Target: Linear to 5 peers

**Implementation:**

```rust
// benches/network_benchmarks.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use wraith_core::Node;
use tokio::runtime::Runtime;

/// Benchmark: Transfer throughput (target: >300 Mbps)
fn bench_transfer_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("transfer_throughput");

    for size in [1_000_000, 10_000_000, 100_000_000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                // Setup two nodes
                let (node1, node2) = setup_two_node_loopback().await;

                // Create test file
                let file_path = format!("/tmp/wraith-bench-{}.dat", size);
                let file_data = vec![0xAA; size];
                tokio::fs::write(&file_path, &file_data).await.unwrap();

                // Measure transfer time
                let start = std::time::Instant::now();

                let transfer_id = node1.send_file(&file_path, node2.node_id()).await.unwrap();
                node1.wait_for_transfer(transfer_id).await.unwrap();

                let duration = start.elapsed();

                // Clean up
                tokio::fs::remove_file(&file_path).await.ok();
                node1.stop().await.unwrap();
                node2.stop().await.unwrap();

                duration
            });
        });
    }

    group.finish();
}

/// Benchmark: Transfer latency (target: <10ms RTT)
fn bench_transfer_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("transfer_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let (node1, node2) = setup_two_node_loopback().await;

            // Measure session establishment time
            let start = std::time::Instant::now();
            let _session_id = node1.establish_session(node2.node_id()).await.unwrap();
            let rtt = start.elapsed();

            node1.stop().await.unwrap();
            node2.stop().await.unwrap();

            black_box(rtt)
        });
    });
}

/// Benchmark: BBR utilization (target: >95%)
fn bench_bbr_utilization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("bbr_utilization", |b| {
        b.to_async(&rt).iter(|| async {
            let (node1, node2) = setup_two_node_loopback().await;

            // Transfer 100 MB file
            let file_path = "/tmp/wraith-bbr-test.dat";
            let file_data = vec![0xBB; 100_000_000];
            tokio::fs::write(file_path, &file_data).await.unwrap();

            // Measure throughput and compare to ideal
            let start = std::time::Instant::now();
            let transfer_id = node1.send_file(file_path, node2.node_id()).await.unwrap();
            node1.wait_for_transfer(transfer_id).await.unwrap();
            let duration = start.elapsed();

            // Calculate utilization
            let throughput_mbps = (100_000_000.0 * 8.0) / duration.as_secs_f64() / 1_000_000.0;
            let utilization = throughput_mbps / 1000.0; // 1 Gbps link

            // Clean up
            tokio::fs::remove_file(file_path).await.ok();
            node1.stop().await.unwrap();
            node2.stop().await.unwrap();

            black_box(utilization)
        });
    });
}

/// Benchmark: Multi-peer speedup (target: linear to 5 peers)
fn bench_multi_peer_speedup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("multi_peer_speedup");

    for num_peers in [1, 2, 3, 5].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(num_peers), num_peers, |b, &num_peers| {
            b.to_async(&rt).iter(|| async {
                // Create receiver node
                let receiver = Node::new_random().unwrap();
                receiver.start().await.unwrap();

                // Create sender nodes
                let mut senders = Vec::new();
                for _ in 0..num_peers {
                    let sender = Node::new_random().unwrap();
                    sender.start().await.unwrap();
                    senders.push(sender);
                }

                // Each sender transfers 10 MB chunk
                let chunk_size = 10_000_000;
                let file_path = format!("/tmp/wraith-multi-peer-{}.dat", num_peers);
                let file_data = vec![0xCC; chunk_size * num_peers];
                tokio::fs::write(&file_path, &file_data).await.unwrap();

                // Measure transfer time with N peers
                let start = std::time::Instant::now();

                // Initiate parallel transfers from all senders
                let mut handles = Vec::new();
                for sender in &senders {
                    let receiver_id = receiver.node_id();
                    let path = file_path.clone();
                    handles.push(sender.send_file(&path, &receiver_id));
                }

                // Wait for all transfers
                for handle in handles {
                    handle.await.unwrap();
                }

                let duration = start.elapsed();

                // Clean up
                tokio::fs::remove_file(&file_path).await.ok();
                for sender in senders {
                    sender.stop().await.unwrap();
                }
                receiver.stop().await.unwrap();

                duration
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_transfer_throughput, bench_transfer_latency, bench_bbr_utilization, bench_multi_peer_speedup);
criterion_main!(benches);
```

**Tasks:**
- [ ] Implement bench_transfer_throughput (1, 10, 100 MB files)
- [ ] Implement bench_transfer_latency (session establishment RTT)
- [ ] Implement bench_bbr_utilization (100 MB file, measure link usage)
- [ ] Implement bench_multi_peer_speedup (1, 2, 3, 5 peers)
- [ ] Run benchmarks on local machine (loopback)
- [ ] Validate targets met:
  - Throughput: >300 Mbps (or document actual)
  - Latency: <10ms RTT (or document actual)
  - BBR: >95% utilization (or document actual)
  - Multi-peer: Linear speedup to 5 peers (or document actual)
- [ ] Update PERFORMANCE_REPORT.md with network benchmark results
- [ ] Document any target misses and optimization opportunities

**Acceptance Criteria:**
- [ ] All 4 network benchmarks implemented
- [ ] Benchmarks run successfully on loopback
- [ ] Results documented in PERFORMANCE_REPORT.md
- [ ] Targets met OR deviations documented with rationale
- [ ] Performance regression tests added to CI (optional)

---

## Sprint 11.2: Network Performance Optimization (Week 3)

**Duration:** 1 week
**Story Points:** 21
**Goal:** Optimize network performance based on benchmark results, address bottlenecks

### 11.2.1: Throughput Optimization (8 SP)

**Objective:** Achieve >300 Mbps throughput on 1 Gbps LAN connections.

**Potential Bottlenecks:**
1. Frame parsing overhead
2. Encryption throughput (XChaCha20-Poly1305)
3. Session lookup (HashMap contention)
4. Worker pool scheduling

**Optimization Strategies:**

#### 1. SIMD-Optimized Frame Parsing

```rust
// crates/wraith-core/src/frame/parser.rs

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Parse frame header (28 bytes) with SIMD
#[cfg(target_arch = "x86_64")]
pub fn parse_frame_header_simd(buf: &[u8]) -> Result<FrameHeader, FrameError> {
    if buf.len() < 28 {
        return Err(FrameError::TooShort);
    }

    unsafe {
        // Load 32 bytes (includes header + padding)
        let data = _mm256_loadu_si256(buf.as_ptr() as *const __m256i);

        // Extract fields with shuffles/shifts
        // (implementation details omitted for brevity)
    }

    // Fallback to scalar parsing
    parse_frame_header_scalar(buf)
}

/// Scalar fallback for non-x86 or when SIMD unavailable
pub fn parse_frame_header_scalar(buf: &[u8]) -> Result<FrameHeader, FrameError> {
    // ... existing scalar implementation ...
}
```

#### 2. Batched Frame Processing

```rust
impl Node {
    /// Process multiple frames in batch
    async fn handle_packet_batch(&self, packets: Vec<(Arc<RwLock<PeerConnection>>, Vec<u8>)>) {
        // Decrypt all frames in parallel
        let frames: Vec<_> = packets.into_iter()
            .map(|(conn, packet)| async move {
                let frame = conn.read().await.decrypt_frame(&packet)?;
                Ok::<_, NodeError>((conn, frame))
            })
            .collect();

        let frames = futures::future::join_all(frames).await;

        // Process frames
        for result in frames {
            if let Ok((conn, frame)) = result {
                self.dispatch_frame(conn, frame).await;
            }
        }
    }
}
```

#### 3. Connection ID Routing Cache

```rust
pub struct RoutingTable {
    routes: Arc<DashMap<u64, Arc<RwLock<PeerConnection>>>>, // Lock-free concurrent map
}

impl RoutingTable {
    /// Lockless lookup with DashMap
    pub fn lookup(&self, connection_id: u64) -> Option<Arc<RwLock<PeerConnection>>> {
        self.routes.get(&connection_id).map(|r| r.value().clone())
    }
}
```

**Tasks:**
- [ ] Profile throughput benchmark (identify bottleneck)
- [ ] Implement SIMD frame parsing (x86_64 AVX2)
- [ ] Implement batched frame processing
- [ ] Replace HashMap routing with DashMap (lock-free)
- [ ] Optimize worker pool (thread-per-core pinning)
- [ ] Re-run throughput benchmark
- [ ] Validate >300 Mbps achieved
- [ ] Document optimizations in PERFORMANCE_REPORT.md

**Acceptance Criteria:**
- [ ] Throughput benchmark: >300 Mbps on 1 Gbps LAN
- [ ] Frame parsing: >2M frames/sec (single core)
- [ ] Zero packet drops under load
- [ ] Optimizations documented

---

### 11.2.2: Latency Optimization (8 SP)

**Objective:** Achieve <10ms RTT for session establishment on LAN.

**Latency Sources:**
1. Noise handshake (3 round trips)
2. Syscall overhead (send/recv)
3. Worker pool scheduling latency

**Optimization Strategies:**

#### 1. Handshake Timeout Tuning

```rust
// crates/wraith-core/src/node/session.rs

pub async fn perform_handshake_initiator<T: Transport>(
    keypair: &NoiseKeypair,
    peer_addr: SocketAddr,
    transport: &T,
) -> Result<(SessionCrypto, SessionId), HandshakeError> {
    // Reduce timeout from 5s → 100ms for LAN
    let timeout = Duration::from_millis(100);

    // ... handshake logic with reduced timeout ...
}
```

#### 2. Zero-Copy Socket Operations

```rust
// Use sendmmsg/recvmmsg for batch operations (Linux)
#[cfg(target_os = "linux")]
async fn send_batch(&self, packets: &[&[u8]]) -> io::Result<()> {
    // sendmmsg allows sending multiple packets in one syscall
    // (reduces syscall overhead)
}
```

#### 3. Fast Session Lookup

```rust
pub struct SessionManager {
    /// Direct PeerId → PeerConnection mapping (no intermediate lookups)
    sessions: Arc<DashMap<PeerId, Arc<RwLock<PeerConnection>>>>,
}
```

**Tasks:**
- [ ] Profile latency benchmark (identify delays)
- [ ] Tune handshake timeouts for LAN (5s → 100ms)
- [ ] Implement zero-copy socket operations (Linux sendmmsg/recvmmsg)
- [ ] Optimize session lookup (direct PeerId map)
- [ ] Re-run latency benchmark
- [ ] Validate <10ms RTT achieved
- [ ] Document optimizations

**Acceptance Criteria:**
- [ ] Session establishment: <10ms RTT on LAN
- [ ] Handshake timeouts appropriate for LAN/WAN
- [ ] Zero syscall overhead with batching
- [ ] Optimizations documented

---

### 11.2.3: BBR Tuning & Multi-Peer Optimization (5 SP)

**Objective:** Achieve >95% BBR link utilization and linear multi-peer speedup.

**BBR Tuning:**

```rust
// crates/wraith-core/src/session/bbr.rs

impl BbrCongestion {
    /// Tune BBR for high-throughput scenarios
    pub fn new_high_throughput() -> Self {
        Self {
            probe_bandwidth_up_cnt: 2,  // More aggressive probing
            probe_bandwidth_down_cnt: 1,
            pacing_gain_probe_up: 1.5,  // 50% increase during probe
            cwnd_gain: 2.0,              // Double CWND
            ..Default::default()
        }
    }
}
```

**Multi-Peer Chunk Assignment:**

```rust
// crates/wraith-core/src/node/chunk_assignment.rs

pub enum AssignmentStrategy {
    FastestFirst, // Assign more chunks to faster peers
}

impl ChunkAssigner {
    fn fastest_first(&self, missing_chunks: &[u64], peers: &[PeerInfo]) -> HashMap<PeerId, Vec<u64>> {
        // Sort peers by throughput
        let mut sorted_peers: Vec<_> = peers.iter()
            .map(|p| (p, p.avg_throughput()))
            .collect();
        sorted_peers.sort_by_key(|(_, throughput)| std::cmp::Reverse(*throughput));

        // Assign chunks proportional to throughput
        let total_throughput: u64 = sorted_peers.iter().map(|(_, t)| t).sum();
        let mut assignments = HashMap::new();

        for (peer, throughput) in sorted_peers {
            let chunk_count = (missing_chunks.len() as f64
                * (*throughput as f64 / total_throughput as f64)) as usize;

            let chunks: Vec<_> = missing_chunks.iter()
                .skip(assignments.values().map(|v: &Vec<u64>| v.len()).sum())
                .take(chunk_count)
                .copied()
                .collect();

            assignments.insert(peer.peer_id, chunks);
        }

        assignments
    }
}
```

**Tasks:**
- [ ] Profile BBR utilization benchmark
- [ ] Tune BBR parameters (probe gains, CWND)
- [ ] Implement FastestFirst chunk assignment strategy
- [ ] Add peer throughput tracking
- [ ] Re-run BBR and multi-peer benchmarks
- [ ] Validate >95% BBR utilization
- [ ] Validate linear multi-peer speedup (up to 5 peers)
- [ ] Document optimizations

**Acceptance Criteria:**
- [ ] BBR benchmark: >95% link utilization
- [ ] Multi-peer benchmark: 3-5x speedup with 5 peers
- [ ] Chunk assignment optimized for peer performance
- [ ] Optimizations documented

---

## Sprint 11.3: Production Hardening (Week 4)

**Duration:** 1 week
**Story Points:** 21
**Goal:** Implement rate limiting, health monitoring, and error recovery for production deployment

### 11.3.1: Rate Limiting & DoS Protection (8 SP)

**Objective:** Protect against resource exhaustion and denial-of-service attacks.

*(Implementation details from Phase 10 Sprint 10.2.1 - omitted for brevity, see phase-10-v1.0.0.md lines 401-576)*

**Key Components:**
- Connection rate limiting (10 attempts/min per IP)
- Packet rate limiting (10K packets/sec per session)
- Bandwidth limiting (100 MB/s per session)
- Global session limit (1000 concurrent)

**Tasks:**
- [ ] Implement RateLimiter with connection, packet, and bandwidth limits
- [ ] Add configurable limits via NodeConfig
- [ ] Integrate into Node packet handling
- [ ] Add metrics for rate limit hits
- [ ] Write 6 tests (connection flood, packet flood, bandwidth flood)

**Acceptance Criteria:**
- [ ] Connection floods blocked (>10 attempts/min per IP)
- [ ] Packet floods blocked (>10K packets/sec per session)
- [ ] Bandwidth floods blocked (>100 MB/s per session)
- [ ] Legitimate traffic unaffected
- [ ] All tests passing

---

### 11.3.2: Resource Limits & Health Monitoring (8 SP)

**Objective:** Enforce memory limits, add health checks, implement graceful degradation.

*(Implementation details from Phase 10 Sprint 10.2.2 - omitted for brevity, see phase-10-v1.0.0.md lines 578-741)*

**Key Components:**
- HealthMonitor with memory/session/transfer limits
- Health status (Healthy, Degraded, Critical)
- Graceful degradation (reduce load at 75% memory)
- Emergency cleanup (close sessions at 90% memory)

**Tasks:**
- [ ] Implement HealthMonitor with resource checks
- [ ] Add health status (Healthy, Degraded, Critical)
- [ ] Implement graceful degradation
- [ ] Implement emergency cleanup
- [ ] Add health metrics endpoint
- [ ] Write 5 tests (health checks, degradation, cleanup)

**Acceptance Criteria:**
- [ ] Memory usage monitored
- [ ] Graceful degradation when >75% memory
- [ ] Emergency cleanup when >90% memory
- [ ] Health metrics available
- [ ] All tests passing

---

### 11.3.3: Error Recovery & Resilience (5 SP)

**Objective:** Comprehensive error handling, automatic retry, circuit breaker patterns.

*(Implementation details from Phase 10 Sprint 10.2.3 - omitted for brevity, see phase-10-v1.0.0.md lines 743-872)*

**Key Components:**
- CircuitBreaker pattern (fail-fast when peer unreachable)
- Automatic retry with exponential backoff
- Session recovery after transient failures
- Handshake packet channeling (fix race condition)

**Tasks:**
- [ ] Implement CircuitBreaker pattern
- [ ] Add automatic retry with exponential backoff
- [ ] Integrate circuit breakers into session establishment
- [ ] Add error logging and metrics
- [ ] Fix handshake race condition: channel incoming handshake packets to pending
      handshake code instead of racing with packet_receive_loop for recv_from()
      (see benches/transfer.rs docstring for bench_transfer_latency_excluded)
- [ ] Re-enable transfer latency benchmarks after handshake fix
- [ ] Write 4 tests (circuit breaker states, retry, backoff)

**Acceptance Criteria:**
- [ ] Circuit breaker prevents cascading failures
- [ ] Automatic retry succeeds after transient failures
- [ ] Exponential backoff prevents thundering herd
- [ ] Handshake packets routed correctly (no race condition)
- [ ] Transfer latency benchmarks pass reliably
- [ ] All tests passing

---

## Sprint 11.4: Advanced Features (Week 5)

**Duration:** 1 week
**Story Points:** 21
**Goal:** Handle edge cases, optimize advanced features, comprehensive testing

### 11.4.1: Resume Robustness (8 SP)

**Objective:** Ensure resume works under all failure scenarios.

*(Implementation details from Phase 10 Sprint 10.3.1 - omitted for brevity, see phase-10-v1.0.0.md lines 880-980)*

**Test Scenarios:**
- Resume after sender restart
- Resume after receiver restart
- Resume after network partition
- Resume after peer change
- Resume with corrupted state

**Tasks:**
- [ ] Implement ResumeState persistence
- [ ] Add resume state validation
- [ ] Handle all 5 failure scenarios
- [ ] Add automatic resume on restart
- [ ] Write 8 tests (5 scenarios + 3 edge cases)

**Acceptance Criteria:**
- [ ] Resume works after sender restart
- [ ] Resume works after receiver restart
- [ ] Resume works after network partition
- [ ] Resume works with peer change
- [ ] Corrupted state detected and handled
- [ ] All tests passing

---

### 11.4.2: Connection Migration Stress Testing (8 SP)

**Objective:** Validate connection migration under all conditions.

*(Implementation details from Phase 10 Sprint 10.3.2 - omitted for brevity, see phase-10-v1.0.0.md lines 982-1056)*

**Test Scenarios:**
- Migration during active transfer
- IPv4 ↔ IPv6 migration
- WiFi ↔ Ethernet handoff
- NAT type changes
- Rapid migrations (deduplication)

**Tasks:**
- [ ] Implement migration during active transfer
- [ ] Handle IPv4 ↔ IPv6 migration
- [ ] Handle interface changes (WiFi ↔ Ethernet)
- [ ] Handle NAT type changes
- [ ] Implement rapid migration deduplication
- [ ] Write 8 tests (all scenarios)

**Acceptance Criteria:**
- [ ] Migration works during active transfer
- [ ] IPv4/IPv6 migration seamless
- [ ] Interface changes handled
- [ ] NAT changes handled
- [ ] Rapid migrations deduplicated
- [ ] All tests passing

---

### 11.4.3: Multi-Peer Optimization (5 SP)

**Objective:** Optimize chunk assignment and peer coordination.

*(Implementation details from Phase 10 Sprint 10.3.3 - omitted for brevity, see phase-10-v1.0.0.md lines 1058-1138)*

**Chunk Assignment Strategies:**
- RoundRobin (simple, even distribution)
- FastestFirst (assign more to faster peers)
- Geographic (consider latency)
- Adaptive (adjust based on performance)

**Tasks:**
- [ ] Implement 4 assignment strategies
- [ ] Add peer performance tracking
- [ ] Benchmark strategies (which is fastest?)
- [ ] Add dynamic rebalancing when peer fails
- [ ] Write 5 tests (each strategy + rebalancing)

**Acceptance Criteria:**
- [ ] All 4 strategies implemented
- [ ] FastestFirst shows measurable improvement
- [ ] Rebalancing works on peer failure
- [ ] Benchmarks show which strategy best
- [ ] All tests passing

---

## Sprint 11.5: XDP Documentation & CLI Implementation (Week 6)

**Duration:** 1 week
**Story Points:** 13
**Goal:** Document XDP unavailability, implement CLI for user-facing functionality

### 11.5.1: XDP Documentation (8 SP)

**Objective:** Document XDP requirements, benefits, and fallback behavior comprehensively.

*(Implementation details from Phase 10 Sprint 10.1.1 Option B - omitted for brevity, see phase-10-v1.0.0.md lines 216-311)*

**Documentation:**
- docs/architecture/xdp-acceleration.md (~500 lines)
- XDP requirements (hardware, kernel, privileges)
- Fallback behavior (UDP graceful degradation)
- Performance comparison (UDP vs XDP)
- Future implementation timeline

**Tasks:**
- [ ] Create docs/architecture/xdp-acceleration.md
- [ ] Document XDP requirements comprehensively
- [ ] Explain fallback behavior
- [ ] Add performance comparison table
- [ ] Document when XDP will be implemented
- [ ] Update README with XDP status
- [ ] Update deployment-guide.md with XDP section

**Acceptance Criteria:**
- [ ] XDP requirements clearly documented
- [ ] Fallback behavior explained
- [ ] Performance expectations set
- [ ] Users understand when XDP available
- [ ] README updated with XDP status

---

### 11.5.2: CLI Implementation (5 SP)

**Objective:** Implement command-line interface for user-facing operations.

*(Consolidated from Technical Debt TD-005 - 1-2 weeks estimated)*

**CLI Commands:**
```bash
# Send file to peer
wraith send <file> --to <peer-id> [--via <relay>]

# Receive files (daemon mode)
wraith receive [--output <dir>] [--daemon]

# List active transfers
wraith status [--transfer <id>]

# List discovered peers
wraith peers [--dht-query <peer-id>]

# Generate identity keypair
wraith keygen [--output <file>]

# Show node information
wraith info
```

**Implementation:**

```rust
// crates/wraith-cli/src/main.rs

use clap::{Parser, Subcommand};
use wraith_core::Node;

#[derive(Parser)]
#[command(name = "wraith")]
#[command(about = "WRAITH Protocol - Secure decentralized file transfer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send file to peer
    Send {
        /// File path to send
        file: String,
        /// Peer ID (hex-encoded Ed25519 public key)
        #[arg(short, long)]
        to: String,
        /// Relay server (optional)
        #[arg(long)]
        via: Option<String>,
    },
    /// Receive files
    Receive {
        /// Output directory for received files
        #[arg(short, long, default_value = "./downloads")]
        output: String,
        /// Run as daemon (background service)
        #[arg(short, long)]
        daemon: bool,
    },
    /// Show transfer status
    Status {
        /// Transfer ID (optional, shows all if omitted)
        #[arg(short, long)]
        transfer: Option<String>,
    },
    /// List discovered peers
    Peers {
        /// Query DHT for specific peer
        #[arg(long)]
        dht_query: Option<String>,
    },
    /// Generate identity keypair
    Keygen {
        /// Output file for identity
        #[arg(short, long, default_value = "~/.wraith/identity")]
        output: String,
    },
    /// Show node information
    Info,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Send { file, to, via } => {
            // ... implementation ...
        }
        Commands::Receive { output, daemon } => {
            // ... implementation ...
        }
        // ... other commands ...
    }

    Ok(())
}
```

**Tasks:**
- [ ] Implement `send` command (file transfer initiation)
- [ ] Implement `receive` command (accept incoming transfers)
- [ ] Implement `status` command (active transfers, connections)
- [ ] Implement `peers` command (DHT query)
- [ ] Implement `keygen` command (identity keypair generation)
- [ ] Implement `info` command (node information)
- [ ] Add progress bars for transfers (indicatif crate)
- [ ] Integration testing with protocol stack
- [ ] User documentation (docs/CLI.md)

**Acceptance Criteria:**
- [ ] All 6 CLI commands functional
- [ ] Progress bars update during transfers
- [ ] Integration tests passing
- [ ] User documentation complete
- [ ] Cross-platform support (Linux, macOS)

---

## Sprint 11.6: Security Validation & Release (Week 7)

**Duration:** 1 week
**Story Points:** 18
**Goal:** Complete documentation, external validation, prepare v1.1.0 release

### 11.6.1: Documentation Completion (8 SP)

**Objective:** Create tutorials, integration guide, troubleshooting, and comparison documentation.

*(Implementation details from Phase 10 Sprint 10.4.1 - omitted for brevity, see phase-10-v1.0.0.md lines 1148-1197)*

**New Documentation:**
1. docs/TUTORIAL.md (~1000 lines) - Step-by-step walkthrough
2. docs/INTEGRATION_GUIDE.md (~800 lines) - Embedding WRAITH in applications
3. docs/TROUBLESHOOTING.md (~600 lines) - Common issues and solutions
4. docs/COMPARISON.md (~500 lines) - WRAITH vs alternatives

**Tasks:**
- [ ] Write TUTORIAL.md with screenshots
- [ ] Write INTEGRATION_GUIDE.md with code examples
- [ ] Write TROUBLESHOOTING.md with solutions
- [ ] Write COMPARISON.md with benchmarks
- [ ] Update README with v1.1.0 status
- [ ] Review all docs for accuracy

**Acceptance Criteria:**
- [ ] Tutorial walks through complete workflow
- [ ] Integration guide has working code examples
- [ ] Troubleshooting covers 20+ scenarios
- [ ] Comparison is fair and accurate
- [ ] All links in docs work
- [ ] cargo doc generates clean documentation

---

### 11.6.2: Security Validation (5 SP)

**Objective:** DIY penetration testing + DPI evasion validation.

*(Implementation details from Phase 10 Sprint 10.4.2 Option B - omitted for brevity, see phase-10-v1.0.0.md lines 1217-1298)*

**Testing:**
- 72-hour fuzzing campaign (frame parser, DHT, crypto)
- Automated penetration testing
- DPI evasion testing (Suricata, nDPI, Zeek)
- Side-channel resistance testing

**Tasks:**
- [ ] Run 72-hour fuzzing campaign
- [ ] Perform penetration testing (automated)
- [ ] Test DPI evasion with Suricata, nDPI, Zeek
- [ ] Test side-channel resistance
- [ ] Document findings
- [ ] Fix any issues found
- [ ] Generate security validation report

**Acceptance Criteria:**
- [ ] Fuzzing: 72 hours, zero crashes
- [ ] Penetration tests: No vulnerabilities found
- [ ] DPI evasion: Not detected by major DPI tools
- [ ] Side-channels: No timing leaks
- [ ] Security report generated

---

### 11.6.3: Reference Client Application (5 SP)

**Objective:** Create simple GUI application demonstrating WRAITH Protocol usage.

*(Implementation details from Phase 10 Sprint 10.4.3 - omitted for brevity, see phase-10-v1.0.0.md lines 1300-1412)*

**WRAITH-Transfer (Basic GUI):**
- Iced-based GUI framework
- Drag-and-drop file selection
- Peer ID input
- Progress bar
- Send/Receive buttons

**Tasks:**
- [ ] Create examples/wraith-transfer-gui
- [ ] Implement basic GUI with iced
- [ ] Add file picker
- [ ] Add progress bar
- [ ] Test on Linux/macOS/Windows
- [ ] Package as standalone executable

**Acceptance Criteria:**
- [ ] GUI application runs
- [ ] Can send file via GUI
- [ ] Progress bar updates
- [ ] Works on Linux/macOS/Windows
- [ ] Packaged as executable

---

## Definition of Done (Phase 11)

### Functionality
- [ ] Packet routing infrastructure complete
- [ ] All 7 deferred integration tests passing
- [ ] Network benchmarks run successfully
- [ ] Performance targets met OR documented
- [ ] Rate limiting functional
- [ ] DoS protection functional
- [ ] Health monitoring functional
- [ ] Resume works under all failure modes
- [ ] Connection migration stress tested
- [ ] Multi-peer optimization complete
- [ ] CLI functional (6 commands)

### Security
- [ ] 72-hour fuzzing complete (zero crashes)
- [ ] DPI evasion validated with real tools
- [ ] Side-channels tested
- [ ] Zero critical security vulnerabilities

### Documentation
- [ ] XDP documentation complete
- [ ] Tutorial complete
- [ ] Integration guide complete
- [ ] Troubleshooting guide complete
- [ ] Comparison guide complete
- [ ] CLI documentation complete
- [ ] All documentation reviewed
- [ ] cargo doc generates clean docs

### Testing
- [ ] All tests passing (target: 1,100+ tests)
- [ ] Integration tests: 47 passing (0 ignored)
- [ ] Resume tests pass (5 scenarios)
- [ ] Migration tests pass (5 scenarios)
- [ ] Multi-peer optimization tested
- [ ] Security validation tests pass

### Quality
- [ ] Zero clippy warnings
- [ ] Zero compilation warnings
- [ ] Zero TODOs in code
- [ ] Technical debt ratio <15%
- [ ] Grade A+ quality maintained

### Release
- [ ] Reference client application working
- [ ] v1.1.0 tag created
- [ ] Release notes written
- [ ] Binaries published
- [ ] Documentation published
- [ ] Announcement prepared

---

## Success Metrics

### Technical Metrics
- [ ] Packet routing: <1μs lookup latency
- [ ] Network throughput: >300 Mbps on 1 Gbps LAN
- [ ] Session latency: <10ms RTT on LAN
- [ ] BBR utilization: >95%
- [ ] Multi-peer speedup: 3-5x with 5 peers
- [ ] Test count: >1,100 (current: 1,025)
- [ ] Test pass rate: 100%
- [ ] Integration tests: 47 passing, 0 ignored

### Functional Metrics
- [ ] Resume success rate: 100% (all 5 scenarios)
- [ ] Migration success rate: 100% (all 5 scenarios)
- [ ] Multi-peer optimization: Measurable improvement
- [ ] CLI: 6 commands functional

### Quality Metrics
- [ ] Zero TODOs in codebase
- [ ] Documentation: 100% complete
- [ ] Technical debt ratio: <15%
- [ ] Grade: A+ maintained

---

## Risk Management

### High-Risk Areas

**1. Packet Routing Performance**
- **Risk:** Routing overhead degrades throughput
- **Mitigation:** Lock-free data structures (DashMap), profiling
- **Contingency:** Accept 10% overhead, optimize in v1.2

**2. Network Benchmark Targets**
- **Risk:** May not achieve >300 Mbps throughput
- **Mitigation:** Profile early, optimize bottlenecks
- **Contingency:** Document actual performance, set realistic targets

**3. Multi-Peer Coordination**
- **Risk:** Coordination overhead reduces speedup
- **Mitigation:** Test with real network conditions
- **Contingency:** Document limitations, defer optimization to v1.2

**4. DPI Evasion Validation**
- **Risk:** May be detected by some tools
- **Mitigation:** Test with multiple DPI tools, iterate
- **Contingency:** Document known limitations

---

## Sprint Summary

| Sprint | Focus | Story Points | Duration |
|--------|-------|--------------|----------|
| **11.1** | Packet Routing & End-to-End Integration | 34 | 2 weeks |
| **11.2** | Network Performance Optimization | 21 | 1 week |
| **11.3** | Production Hardening | 21 | 1 week |
| **11.4** | Advanced Features | 21 | 1 week |
| **11.5** | XDP Documentation & CLI | 13 | 1 week |
| **11.6** | Security Validation & Release | 18 | 1 week |
| **Total** | | **128 SP** | **7 weeks** |

---

## Critical Path

```
Phase 10 Sessions 2-4 Complete
    ↓
Sprint 11.1: Packet Routing (CRITICAL - unblocks everything)
    ↓
Sprint 11.2: Network Performance Optimization
    ↓
Sprint 11.3: Production Hardening ─┐
    ↓                                │
Sprint 11.4: Advanced Features ─────┤
    ↓                                │
Sprint 11.5: XDP Docs & CLI ────────┤
    ↓                                │
Sprint 11.6: Security & Release ←───┘
    ↓
v1.1.0 RELEASE
```

**Critical Path Duration:** 7 weeks

**Blocker:** Sprint 11.1 (packet routing) must complete before network benchmarks and deferred tests can be validated.

---

## Completion Checklist

- [ ] Sprint 11.1: Packet Routing & End-to-End Integration (34 SP)
- [ ] Sprint 11.2: Network Performance Optimization (21 SP)
- [ ] Sprint 11.3: Production Hardening (21 SP)
- [ ] Sprint 11.4: Advanced Features (21 SP)
- [ ] Sprint 11.5: XDP Documentation & CLI (13 SP)
- [ ] Sprint 11.6: Security Validation & Release (18 SP)
- [ ] All acceptance criteria met
- [ ] All documentation complete
- [ ] Security validation passed
- [ ] README updated (v1.1.0 status)
- [ ] CHANGELOG.md updated
- [ ] Release v1.1.0 prepared
- [ ] Announcement written
- [ ] Binaries published

**Estimated Completion:** 7 weeks (full-time) or 14 weeks (part-time)

---

## Notes

### Phase 10 vs Phase 11 Scope

**Phase 10 Delivered (~60% of original plan):**
- Component integration (transport, crypto, discovery, NAT, obfuscation, file transfer)
- File operations benchmarking
- 40 integration tests passing
- Foundation for end-to-end protocol

**Phase 10 Deferred (now in Phase 11):**
- Packet routing infrastructure (newly identified gap)
- Network performance benchmarks
- Production hardening (rate limiting, health monitoring, resilience)
- Advanced features (resume, migration, multi-peer optimization)
- XDP documentation
- Complete documentation set
- Security validation
- Reference client

**Why the Split:**
Phase 10 Session 4 benchmarking revealed the packet routing infrastructure gap. Rather than rushing to complete all original Phase 10 work with this blocker, Phase 11 addresses the infrastructure gap first, then completes all deferred items systematically.

### v1.1.0 Rationale

The original Phase 10 targeted v1.0.0. However, given:
1. Packet routing infrastructure gap discovered
2. Additional testing and validation needed
3. More realistic timeline (7 weeks vs rushing)

Phase 11 targets **v1.1.0** as a production-complete release that includes:
- Full end-to-end protocol integration
- Comprehensive performance validation
- Production hardening
- Complete documentation
- Security validation

This provides a more realistic timeline and ensures quality over speed.

---

## Dependencies & Blockers

### External Dependencies
- **Testing Hardware:** 1 Gbps LAN for network benchmarks (available)
- **DPI Tools:** Suricata, nDPI, Zeek (open-source, installable)
- **Fuzzing Infrastructure:** AFL++, cargo-fuzz (available)

### Potential Blockers
1. **Packet Routing Complexity:** May take longer than 2 weeks if unforeseen issues arise
   - **Mitigation:** Start with minimal implementation, iterate
   - **Contingency:** Extend Sprint 11.1 by 1 week if needed

2. **Performance Target Misses:** May not achieve all targets on first attempt
   - **Mitigation:** Profile early, optimize iteratively
   - **Contingency:** Document actual performance, defer optimization to v1.2

3. **DPI Detection:** Some DPI tools may detect protocol
   - **Mitigation:** Test with multiple tools, iterate on mimicry
   - **Contingency:** Document known limitations

---

## Next Steps

1. **Sprint 11.1 Kickoff:** Implement packet routing infrastructure (21 SP)
2. **Un-Ignore Tests:** Enable 7 deferred integration tests (8 SP)
3. **Network Benchmarks:** Run throughput, latency, BBR, multi-peer tests (5 SP)
4. **Performance Optimization:** Address any bottlenecks discovered
5. **Production Hardening:** Rate limiting, health monitoring, resilience
6. **Advanced Features:** Resume, migration, multi-peer optimization
7. **Documentation:** XDP, tutorials, integration guide, troubleshooting, CLI
8. **Security Validation:** Fuzzing, DPI testing, penetration testing
9. **Reference Client:** Minimal GUI application
10. **v1.1.0 Release:** Tag, publish, announce

---

**WRAITH Protocol v1.1.0 - PRODUCTION COMPLETE!**

After Phase 11, WRAITH Protocol will be **100% complete**, production-hardened, comprehensively documented, and ready for public release. All protocol features implemented, all documentation complete, all tests passing, zero TODOs remaining.

**Total Project:** 947 SP (original Phases 1-7) + 85 SP (Phase 9) + ~60% Phase 10 + 128 SP (Phase 11) = **~1,100 SP delivered**

**Public Release Ready:** ✅
