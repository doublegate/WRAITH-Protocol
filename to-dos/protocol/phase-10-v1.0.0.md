# Phase 10: v1.0.0 - Production Complete & Public Release

**Target:** v1.0.0 Production Release - 100% Protocol Implementation
**Estimated Effort:** 93 Story Points (~4-5 weeks)
**Prerequisites:** Phase 9 (v0.9.0) complete - Working end-to-end protocol

---

## Overview

Phase 10 transforms WRAITH Protocol from a working beta (v0.9.0) into a production-complete implementation ready for public release. While v0.9.0 delivers a functional protocol capable of secure file transfers, v1.0.0 adds the polish, optional features, hardening, and comprehensive documentation needed for widespread adoption.

**What v1.0.0 Achieves:**
- **Production Hardened:** Rate limiting, DoS protection, resource limits
- **Feature Complete:** All optional features implemented or documented
- **Security Validated:** External audit passed, penetration tested
- **Fully Documented:** Tutorials, troubleshooting, integration guides
- **Reference Implementation:** At least one client application showcasing the protocol
- **Zero TODOs:** All code complete, no placeholders or deferred work

**v0.9.0 → v1.0.0 Delta:**
- ✅ v0.9.0: Working protocol (files transfer end-to-end)
- ➕ XDP acceleration (or documentation why unavailable)
- ➕ Production hardening (security, reliability, observability)
- ➕ Advanced features (optimization, edge cases)
- ➕ External validation (security audit, DPI testing)
- ➕ Complete documentation (tutorials, guides, reference)
- ➕ Reference client (demonstrates protocol usage)
- = ✅ v1.0.0: Production complete

---

## Sprint 10.1: XDP Acceleration & Performance Validation (Weeks 1-2)

**Duration:** 2 weeks
**Story Points:** 34
**Goal:** Implement XDP packet filtering OR thoroughly document why unavailable, validate all performance targets

### 10.1.1: XDP Implementation (21 SP) OR Documentation (8 SP)

**Objective:** Complete wraith-xdp crate with eBPF programs for packet filtering.

**Option A: Full XDP Implementation (21 SP)**

*If eBPF toolchain and hardware available:*

```rust
// crates/wraith-xdp/src/lib.rs

use libbpf_rs::{Program, Map, Object};
use std::path::Path;

/// XDP packet filter
///
/// Performs early packet filtering in kernel before copying to userspace.
/// Filters on:
/// - Connection ID (CID) - Only packets for active sessions
/// - Invalid frames - Drop malformed packets immediately
/// - Rate limiting - Drop excessive packets from single source
///
/// **Requirements:**
/// - Linux kernel 5.7+ (for AF_XDP)
/// - Capable NIC (Intel X710, Mellanox ConnectX-5+)
/// - Root privileges
/// - eBPF toolchain (clang, llvm-objdump)
pub struct XdpFilter {
    program: Program,
    cid_map: Map,
    rate_limit_map: Map,
}

impl XdpFilter {
    /// Load XDP program on interface
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wraith_xdp::XdpFilter;
    /// let filter = XdpFilter::load("eth0")?;
    /// filter.attach()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn load(interface: &str) -> Result<Self, XdpError>;

    /// Attach XDP program to interface
    pub fn attach(&self) -> Result<(), XdpError>;

    /// Add CID to allow-list
    pub fn add_cid(&mut self, cid: u64) -> Result<(), XdpError>;

    /// Remove CID from allow-list
    pub fn remove_cid(&mut self, cid: u64) -> Result<(), XdpError>;

    /// Get statistics (packets passed, dropped)
    pub fn get_stats(&self) -> Result<XdpStats, XdpError>;
}

#[derive(Debug, Clone)]
pub struct XdpStats {
    pub packets_passed: u64,
    pub packets_dropped: u64,
    pub packets_invalid: u64,
    pub packets_rate_limited: u64,
}
```

**eBPF Program (BPF C):**
```c
// crates/wraith-xdp/src/bpf/filter.bpf.c

#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/udp.h>
#include <bpf/bpf_helpers.h>

// CID allow-list map
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __type(key, __u64);    // CID
    __type(value, __u8);   // 1 = allowed
    __uint(max_entries, 65536);
} cid_map SEC(".maps");

// Rate limiting map (src IP -> packet count)
struct {
    __uint(type, BPF_MAP_TYPE_LRU_HASH);
    __type(key, __u32);    // Source IP
    __type(value, __u32);  // Packet count in window
    __uint(max_entries, 10000);
} rate_limit_map SEC(".maps");

SEC("xdp")
int wraith_filter(struct xdp_md *ctx) {
    void *data_end = (void *)(long)ctx->data_end;
    void *data = (void *)(long)ctx->data;

    // Parse Ethernet header
    struct ethhdr *eth = data;
    if ((void *)(eth + 1) > data_end)
        return XDP_DROP;

    // Only process IP packets
    if (eth->h_proto != __constant_htons(ETH_P_IP))
        return XDP_PASS;

    // Parse IP header
    struct iphdr *ip = (void *)(eth + 1);
    if ((void *)(ip + 1) > data_end)
        return XDP_DROP;

    // Only process UDP
    if (ip->protocol != IPPROTO_UDP)
        return XDP_PASS;

    // Parse UDP header
    struct udphdr *udp = (void *)(ip + 1);
    if ((void *)(udp + 1) > data_end)
        return XDP_DROP;

    // Extract WRAITH CID (first 8 bytes of payload)
    __u64 *cid_ptr = (void *)(udp + 1);
    if ((void *)(cid_ptr + 1) > data_end)
        return XDP_DROP;

    __u64 cid = *cid_ptr;

    // Check if CID is in allow-list
    __u8 *allowed = bpf_map_lookup_elem(&cid_map, &cid);
    if (!allowed)
        return XDP_DROP;  // Unknown CID

    // Rate limiting: Check packet count from source IP
    __u32 src_ip = ip->saddr;
    __u32 *count = bpf_map_lookup_elem(&rate_limit_map, &src_ip);

    if (count) {
        if (*count > 10000) {  // Max 10K packets/sec per IP
            return XDP_DROP;
        }
        __sync_fetch_and_add(count, 1);
    } else {
        __u32 initial = 1;
        bpf_map_update_elem(&rate_limit_map, &src_ip, &initial, BPF_ANY);
    }

    return XDP_PASS;  // Allow packet to AF_XDP socket
}

char _license[] SEC("license") = "GPL";
```

**Tasks (Option A - Full Implementation):**
- [ ] Set up eBPF build system (clang, libbpf)
- [ ] Implement filter.bpf.c with CID filtering and rate limiting
- [ ] Implement XdpFilter Rust wrapper
- [ ] Add CID management (add/remove)
- [ ] Integrate with AF_XDP transport (crates/wraith-transport/src/af_xdp.rs)
- [ ] Add XDP statistics collection
- [ ] Write 8 tests (load, attach, CID filtering, rate limiting, stats)
- [ ] Document hardware requirements

**Acceptance Criteria (Option A):**
- [ ] XDP program loads and attaches successfully
- [ ] CID filtering drops unknown sessions
- [ ] Rate limiting protects against floods
- [ ] Integration with AF_XDP transport working
- [ ] Statistics reported correctly
- [ ] Tests pass on XDP-capable hardware

---

**Option B: XDP Documentation (8 SP)**

*If eBPF toolchain or hardware unavailable:*

**Objective:** Document XDP requirements, benefits, and fallback behavior comprehensively.

```markdown
# docs/architecture/xdp-acceleration.md

## XDP Acceleration in WRAITH Protocol

### Overview

WRAITH Protocol includes optional XDP (eXpress Data Path) acceleration for
kernel-bypass packet processing. XDP provides:

- **10-40 Gbps throughput** (vs 1-3 Gbps UDP)
- **Sub-microsecond latency** (vs 10-100μs UDP)
- **Zero-copy packet processing** (no kernel copies)
- **Early packet filtering** (drop invalid packets in kernel)

However, XDP requires:
- Linux kernel 5.7+
- XDP-capable NIC (Intel X710, Mellanox ConnectX-5+)
- Root privileges
- eBPF toolchain (clang 10+)

### Why XDP Unavailable

**Current Status:** XDP implementation deferred due to:

1. **Hardware Requirement:** Requires specialized NICs not universally available
2. **Privilege Requirement:** Needs root access, limits deployment scenarios
3. **Platform Limitation:** Linux-only, breaks cross-platform goal
4. **Development Cost:** eBPF toolchain setup, testing infrastructure

### Fallback Behavior

WRAITH gracefully falls back to UDP when XDP unavailable:

```rust
if config.enable_xdp {
    match AfXdpTransport::new(&config).await {
        Ok(transport) => return Ok(transport),
        Err(e) => tracing::warn!("XDP unavailable: {}, using UDP", e),
    }
}

UdpTransport::new(&config).await  // Fallback
```

**UDP Performance:**
- Throughput: 300-950 Mbps (1 Gbps links)
- Latency: 10-100μs
- Sufficient for most use cases

### Future Implementation

XDP acceleration will be implemented when:
- Access to XDP-capable hardware confirmed
- eBPF toolchain available in CI
- Sufficient demand from users

**Estimated Effort:** 21 SP (~2 weeks)
**Priority:** Low (UDP fallback sufficient)

### Performance Comparison

| Metric | UDP Fallback | XDP Acceleration |
|--------|--------------|------------------|
| Throughput (1GbE) | 300-950 Mbps | N/A (limited by link) |
| Throughput (10GbE) | 1-3 Gbps | 9+ Gbps |
| Latency | 10-100μs | <1μs |
| CPU Usage (10Gbps) | 80-90% (8 cores) | 40-60% (8 cores) |
| Kernel Copies | 2 (NIC→kernel→user) | 0 (DMA direct) |

### References

- [XDP Tutorial](https://github.com/xdp-project/xdp-tutorial)
- [AF_XDP Documentation](https://www.kernel.org/doc/html/latest/networking/af_xdp.html)
- [libbpf Documentation](https://github.com/libbpf/libbpf)
```

**Tasks (Option B - Documentation):**
- [ ] Create docs/architecture/xdp-acceleration.md
- [ ] Document XDP requirements comprehensively
- [ ] Explain fallback behavior
- [ ] Add performance comparison table
- [ ] Document when XDP will be implemented
- [ ] Update README with XDP status
- [ ] Update deployment-guide.md with XDP section

**Acceptance Criteria (Option B):**
- [ ] XDP requirements clearly documented
- [ ] Fallback behavior explained
- [ ] Performance expectations set
- [ ] Users understand when XDP available
- [ ] README updated with XDP status

---

### 10.1.2: Performance Validation & Optimization (13 SP)

**Objective:** Validate all performance targets met, optimize bottlenecks if needed.

**Performance Targets (from Phase 9):**
- ✅ Throughput: >300 Mbps on 1 Gbps LAN
- ✅ Latency: <10ms RTT on LAN
- ✅ BBR utilization: >95% link utilization
- ✅ Multi-peer speedup: Linear to 5 peers

**Validation Tasks:**
```bash
#!/bin/bash
# scripts/validate_performance.sh

echo "=== WRAITH Protocol Performance Validation ==="

# 1. Throughput test (1 Gbps LAN)
echo "1. Testing throughput (target: >300 Mbps)..."
cargo bench --bench transfer -- bench_transfer_throughput

# 2. Latency test
echo "2. Testing latency (target: <10ms RTT)..."
cargo bench --bench transfer -- bench_transfer_latency

# 3. BBR utilization test
echo "3. Testing BBR utilization (target: >95%)..."
cargo bench --bench transfer -- bench_bbr_utilization

# 4. Multi-peer speedup test
echo "4. Testing multi-peer speedup (target: linear to 5 peers)..."
cargo bench --bench transfer -- bench_multi_peer_speedup

# 5. Generate report
echo "Generating performance report..."
cargo xtask perf-report > docs/PERFORMANCE_REPORT.md
```

**Optimization Targets:**

If benchmarks reveal bottlenecks:

1. **Throughput < 300 Mbps:**
   - Profile with `perf` to find hotspots
   - Check SIMD usage in frame parsing
   - Optimize allocation-heavy paths
   - Consider buffer pools

2. **Latency > 10ms:**
   - Reduce syscall overhead
   - Check worker pool contention
   - Optimize session lookup (use HashMap)

3. **BBR < 95%:**
   - Tune BBR parameters (probe_bandwidth_up_cnt)
   - Check pacing accuracy
   - Review congestion window calculations

4. **Multi-peer < linear:**
   - Optimize chunk assignment algorithm
   - Check peer coordination overhead
   - Review lock contention in TransferSession

**Tasks:**
- [ ] Run all 4 performance benchmarks
- [ ] Validate targets met
- [ ] Profile if targets missed
- [ ] Optimize identified bottlenecks
- [ ] Document actual performance in README
- [ ] Create PERFORMANCE_REPORT.md with results
- [ ] Add performance regression tests to CI (optional)

**Acceptance Criteria:**
- [ ] All 4 benchmarks run successfully
- [ ] Performance targets met OR documented why not
- [ ] Bottlenecks identified and optimized
- [ ] Performance report generated
- [ ] Results published in documentation

---

## Sprint 10.2: Production Hardening (Week 2-3)

**Duration:** 1 week
**Story Points:** 21
**Goal:** Implement security hardening, reliability features, and observability for production deployment

### 10.2.1: Rate Limiting & DoS Protection (8 SP)

**Objective:** Protect against resource exhaustion and denial-of-service attacks.

```rust
// crates/wraith-core/src/node/rate_limit.rs

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Rate limiter for incoming connections and requests
pub struct RateLimiter {
    /// Connection attempts per IP
    connection_limits: HashMap<IpAddr, ConnectionLimit>,

    /// Packet rate per session
    packet_limits: HashMap<SessionId, PacketLimit>,

    /// Global limits
    config: RateLimitConfig,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Max connection attempts per IP per minute
    pub max_connections_per_ip: u32,

    /// Max packets per session per second
    pub max_packets_per_session: u32,

    /// Max concurrent sessions total
    pub max_concurrent_sessions: usize,

    /// Max bandwidth per session (bytes/sec)
    pub max_bandwidth_per_session: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_connections_per_ip: 10,      // 10 attempts/min
            max_packets_per_session: 10000,  // 10K packets/sec
            max_concurrent_sessions: 1000,    // 1K sessions
            max_bandwidth_per_session: 100_000_000, // 100 MB/s
        }
    }
}

impl RateLimiter {
    /// Check if connection attempt allowed
    pub fn check_connection(&mut self, ip: IpAddr) -> Result<(), RateLimitError> {
        let limit = self.connection_limits.entry(ip).or_insert_with(ConnectionLimit::new);

        limit.prune_old_attempts();

        if limit.attempts.len() >= self.config.max_connections_per_ip as usize {
            return Err(RateLimitError::TooManyConnections);
        }

        limit.attempts.push(Instant::now());
        Ok(())
    }

    /// Check if packet allowed for session
    pub fn check_packet(&mut self, session_id: SessionId) -> Result<(), RateLimitError> {
        let limit = self.packet_limits.entry(session_id).or_insert_with(PacketLimit::new);

        limit.update();

        if limit.count > self.config.max_packets_per_session {
            return Err(RateLimitError::TooManyPackets);
        }

        limit.count += 1;
        Ok(())
    }

    /// Check if bandwidth limit exceeded
    pub fn check_bandwidth(
        &mut self,
        session_id: SessionId,
        bytes: u64,
    ) -> Result<(), RateLimitError> {
        let limit = self.packet_limits.entry(session_id).or_insert_with(PacketLimit::new);

        limit.update();

        if limit.bytes_this_second + bytes > self.config.max_bandwidth_per_session {
            return Err(RateLimitError::BandwidthExceeded);
        }

        limit.bytes_this_second += bytes;
        Ok(())
    }
}

struct ConnectionLimit {
    attempts: Vec<Instant>,
}

impl ConnectionLimit {
    fn new() -> Self {
        Self { attempts: Vec::new() }
    }

    fn prune_old_attempts(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(60);
        self.attempts.retain(|&t| t > cutoff);
    }
}

struct PacketLimit {
    count: u32,
    bytes_this_second: u64,
    window_start: Instant,
}

impl PacketLimit {
    fn new() -> Self {
        Self {
            count: 0,
            bytes_this_second: 0,
            window_start: Instant::now(),
        }
    }

    fn update(&mut self) {
        if self.window_start.elapsed() > Duration::from_secs(1) {
            self.count = 0;
            self.bytes_this_second = 0;
            self.window_start = Instant::now();
        }
    }
}
```

**Integration into Node:**
```rust
impl Node {
    async fn handle_incoming_connection(&self, addr: SocketAddr) -> Result<(), NodeError> {
        // Check rate limit
        self.rate_limiter.write().await
            .check_connection(addr.ip())?;

        // ... proceed with connection ...
    }

    async fn handle_packet(&self, session_id: SessionId, packet: &[u8]) -> Result<(), NodeError> {
        // Check packet rate
        self.rate_limiter.write().await
            .check_packet(session_id)?;

        // Check bandwidth
        self.rate_limiter.write().await
            .check_bandwidth(session_id, packet.len() as u64)?;

        // ... process packet ...
    }
}
```

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
- [ ] Tests verify all limits

---

### 10.2.2: Resource Limits & Health Monitoring (8 SP)

**Objective:** Enforce memory limits, add health checks, implement graceful degradation.

```rust
// crates/wraith-core/src/node/health.rs

use sysinfo::{System, SystemExt, ProcessExt};

/// Health monitor for node
pub struct HealthMonitor {
    config: HealthConfig,
    system: System,
}

#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Max memory usage (bytes)
    pub max_memory_bytes: u64,

    /// Max sessions
    pub max_sessions: usize,

    /// Max transfers
    pub max_transfers: usize,

    /// Health check interval
    pub check_interval: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}

impl HealthMonitor {
    /// Check node health
    pub fn check_health(&mut self) -> HealthStatus {
        self.system.refresh_all();

        let memory_usage = self.get_memory_usage();
        let memory_pct = (memory_usage as f64 / self.config.max_memory_bytes as f64) * 100.0;

        if memory_pct > 90.0 {
            HealthStatus::Critical
        } else if memory_pct > 75.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Get memory usage in bytes
    pub fn get_memory_usage(&self) -> u64 {
        let pid = std::process::id();

        if let Some(process) = self.system.process(pid.into()) {
            process.memory() * 1024  // Convert KiB to bytes
        } else {
            0
        }
    }

    /// Get health metrics
    pub fn get_metrics(&self) -> HealthMetrics {
        HealthMetrics {
            memory_bytes: self.get_memory_usage(),
            max_memory_bytes: self.config.max_memory_bytes,
            active_sessions: 0,  // Filled by caller
            max_sessions: self.config.max_sessions,
            active_transfers: 0,  // Filled by caller
            max_transfers: self.config.max_transfers,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthMetrics {
    pub memory_bytes: u64,
    pub max_memory_bytes: u64,
    pub active_sessions: usize,
    pub max_sessions: usize,
    pub active_transfers: usize,
    pub max_transfers: usize,
}

impl Node {
    /// Start health monitoring
    pub async fn start_health_monitoring(&self) {
        let node = Arc::clone(&self.inner);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let status = node.health_monitor.write().await.check_health();

                match status {
                    HealthStatus::Healthy => {},
                    HealthStatus::Degraded => {
                        tracing::warn!("Node health degraded");
                        // Reduce resource usage
                        node.reduce_load().await;
                    }
                    HealthStatus::Critical => {
                        tracing::error!("Node health critical");
                        // Emergency cleanup
                        node.emergency_cleanup().await;
                    }
                }
            }
        });
    }

    /// Reduce load when degraded
    async fn reduce_load(&self) {
        // Reject new connections
        self.accepting_connections.store(false, Ordering::Relaxed);

        // Reduce worker threads
        // ... implementation ...
    }

    /// Emergency cleanup when critical
    async fn emergency_cleanup(&self) {
        // Close oldest sessions
        let mut sessions = self.sessions.write().await;

        let to_close: Vec<_> = sessions.iter()
            .filter(|(_, s)| s.idle_time() > Duration::from_secs(60))
            .map(|(id, _)| *id)
            .collect();

        for session_id in to_close {
            sessions.remove(&session_id);
        }

        tracing::info!("Emergency cleanup: closed {} sessions", to_close.len());
    }
}
```

**Tasks:**
- [ ] Implement HealthMonitor with resource checks
- [ ] Add health status (Healthy, Degraded, Critical)
- [ ] Implement graceful degradation (reduce load when degraded)
- [ ] Implement emergency cleanup (close sessions when critical)
- [ ] Add health metrics endpoint
- [ ] Write 5 tests (health checks, degradation, cleanup)

**Acceptance Criteria:**
- [ ] Memory usage monitored
- [ ] Graceful degradation when >75% memory
- [ ] Emergency cleanup when >90% memory
- [ ] Health metrics available
- [ ] Tests verify all states

---

### 10.2.3: Error Recovery & Resilience (5 SP)

**Objective:** Comprehensive error handling, automatic retry, circuit breaker patterns.

```rust
// crates/wraith-core/src/node/resilience.rs

/// Circuit breaker for peer connections
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    last_failure: Option<Instant>,
    config: CircuitConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing, reject requests
    HalfOpen,    // Testing if recovered
}

#[derive(Debug, Clone)]
pub struct CircuitConfig {
    /// Failures before opening circuit
    pub failure_threshold: u32,

    /// Time before trying again
    pub reset_timeout: Duration,

    /// Successes needed to close circuit
    pub success_threshold: u32,
}

impl CircuitBreaker {
    /// Check if request allowed
    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if should transition to half-open
                if let Some(last_failure) = self.last_failure {
                    if last_failure.elapsed() > self.config.reset_timeout {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record success
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                // Close circuit after success
                self.state = CircuitState::Closed;
                self.failure_count = 0;
            }
            _ => {}
        }
    }

    /// Record failure
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        if self.failure_count >= self.config.failure_threshold {
            self.state = CircuitState::Open;
        }
    }
}

impl Node {
    /// Establish session with retry and circuit breaker
    pub async fn establish_session_with_retry(
        &self,
        peer_id: &PeerId,
    ) -> Result<SessionId, SessionError> {
        let max_retries = 3;
        let mut attempt = 0;

        loop {
            // Check circuit breaker
            if !self.circuit_breakers.get(peer_id).allow_request() {
                return Err(SessionError::CircuitOpen);
            }

            match self.establish_session(peer_id).await {
                Ok(session_id) => {
                    self.circuit_breakers.get_mut(peer_id).record_success();
                    return Ok(session_id);
                }
                Err(e) => {
                    self.circuit_breakers.get_mut(peer_id).record_failure();

                    attempt += 1;
                    if attempt >= max_retries {
                        return Err(e);
                    }

                    // Exponential backoff
                    let delay = Duration::from_millis(100 * 2_u64.pow(attempt));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}
```

**Tasks:**
- [ ] Implement CircuitBreaker pattern
- [ ] Add automatic retry with exponential backoff
- [ ] Integrate circuit breakers into session establishment
- [ ] Add error logging and metrics
- [ ] Write 4 tests (circuit breaker states, retry, backoff)

**Acceptance Criteria:**
- [ ] Circuit breaker prevents cascading failures
- [ ] Automatic retry succeeds after transient failures
- [ ] Exponential backoff prevents thundering herd
- [ ] Tests verify all states

---

## Sprint 10.3: Advanced Features & Edge Cases (Week 3-4)

**Duration:** 1 week
**Story Points:** 21
**Goal:** Handle edge cases, optimize advanced features, comprehensive testing

### 10.3.1: Resume Robustness (8 SP)

**Objective:** Ensure resume works under all failure scenarios.

**Test Scenarios:**
```rust
// tests/resume_tests.rs

#[tokio::test]
async fn test_resume_after_sender_restart() {
    // Transfer 100 MB, crash sender at 50 MB, restart, resume
}

#[tokio::test]
async fn test_resume_after_receiver_restart() {
    // Transfer 100 MB, crash receiver at 50 MB, restart, resume
}

#[tokio::test]
async fn test_resume_after_network_partition() {
    // Transfer 100 MB, drop all packets for 30 seconds, resume
}

#[tokio::test]
async fn test_resume_after_peer_change() {
    // Transfer from peer A, peer A dies, resume from peer B
}

#[tokio::test]
async fn test_resume_with_corrupted_state() {
    // Corrupt .wraith-resume file, should detect and restart transfer
}
```

**Resume State Persistence:**
```rust
// crates/wraith-files/src/resume.rs

use serde::{Serialize, Deserialize};

/// Resume state file (.wraith-resume)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeState {
    pub transfer_id: TransferId,
    pub file_path: PathBuf,
    pub total_size: u64,
    pub chunk_size: usize,
    pub tree_hash: FileTreeHash,
    pub received_chunks: HashSet<u64>,
    pub peers: Vec<PeerId>,
    pub last_updated: SystemTime,
}

impl ResumeState {
    /// Save to disk
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let data = bincode::serialize(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Load from disk
    pub fn load(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let data = std::fs::read(path)?;
        let state = bincode::deserialize(&data)?;
        Ok(state)
    }

    /// Validate state (check hash, file exists, etc.)
    pub fn validate(&self) -> Result<(), ResumeError> {
        // Check file exists
        if !self.file_path.exists() {
            return Err(ResumeError::FileNotFound);
        }

        // Verify file size matches
        let metadata = std::fs::metadata(&self.file_path)?;
        if metadata.len() != self.total_size {
            return Err(ResumeError::SizeMismatch);
        }

        Ok(())
    }
}
```

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
- [ ] All tests pass

---

### 10.3.2: Connection Migration Stress Testing (8 SP)

**Objective:** Validate connection migration under all conditions.

**Test Scenarios:**
```rust
// tests/migration_tests.rs

#[tokio::test]
async fn test_migration_during_transfer() {
    // Transfer 100 MB, change IP mid-transfer, verify continues
}

#[tokio::test]
async fn test_migration_ipv4_to_ipv6() {
    // Start on IPv4, migrate to IPv6
}

#[tokio::test]
async fn test_migration_wifi_to_ethernet() {
    // Simulate WiFi to Ethernet handoff
}

#[tokio::test]
async fn test_migration_with_nat_change() {
    // NAT type changes during transfer
}

#[tokio::test]
async fn test_rapid_migrations() {
    // Multiple IP changes in quick succession
}
```

**Enhanced Migration:**
```rust
impl Node {
    /// Handle rapid migrations (multiple IP changes)
    async fn handle_rapid_migration(&self, session_id: &SessionId) -> Result<(), SessionError> {
        let session = self.sessions.read().await
            .get(session_id)
            .ok_or(SessionError::SessionNotFound)?;

        // Deduplicate rapid migration attempts
        let last_migration = session.last_migration_time();

        if last_migration.elapsed() < Duration::from_secs(5) {
            // Too rapid, ignore
            return Ok(());
        }

        // Proceed with migration
        self.migrate_connection(session_id, new_addr).await
    }
}
```

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
- [ ] All tests pass

---

### 10.3.3: Multi-Peer Optimization (5 SP)

**Objective:** Optimize chunk assignment and peer coordination.

**Smart Chunk Assignment:**
```rust
// crates/wraith-core/src/node/chunk_assignment.rs

/// Optimized chunk assignment algorithm
pub struct ChunkAssigner {
    strategy: AssignmentStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum AssignmentStrategy {
    RoundRobin,       // Simple, even distribution
    FastestFirst,     // Assign more to faster peers
    Geographic,       // Consider peer location/latency
    Adaptive,         // Adjust based on performance
}

impl ChunkAssigner {
    /// Assign chunks to peers
    pub fn assign_chunks(
        &self,
        missing_chunks: &[u64],
        peers: &[PeerInfo],
    ) -> HashMap<PeerId, Vec<u64>> {
        match self.strategy {
            AssignmentStrategy::RoundRobin => self.round_robin(missing_chunks, peers),
            AssignmentStrategy::FastestFirst => self.fastest_first(missing_chunks, peers),
            AssignmentStrategy::Geographic => self.geographic(missing_chunks, peers),
            AssignmentStrategy::Adaptive => self.adaptive(missing_chunks, peers),
        }
    }

    fn fastest_first(
        &self,
        missing_chunks: &[u64],
        peers: &[PeerInfo],
    ) -> HashMap<PeerId, Vec<u64>> {
        // Sort peers by throughput
        let mut sorted_peers: Vec<_> = peers.iter()
            .map(|p| (p, p.avg_throughput()))
            .collect();
        sorted_peers.sort_by_key(|(_, throughput)| std::cmp::Reverse(*throughput));

        // Assign more chunks to faster peers
        let mut assignments = HashMap::new();
        let total_throughput: u64 = sorted_peers.iter().map(|(_, t)| t).sum();

        for (peer, throughput) in sorted_peers {
            let chunk_count = (missing_chunks.len() as f64
                * (*throughput as f64 / total_throughput as f64)) as usize;

            let chunks: Vec<_> = missing_chunks.iter()
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
- [ ] Tests verify all strategies

---

## Sprint 10.4: Documentation & Release Preparation (Week 4-5)

**Duration:** 1 week
**Story Points:** 17
**Goal:** Complete documentation, external validation, prepare v1.0.0 release

### 10.4.1: Documentation Completion (8 SP)

**Objective:** Create tutorials, integration guide, troubleshooting, and comparison documentation.

**New Documentation:**

1. **docs/TUTORIAL.md** (~1000 lines)
   - Step-by-step first transfer
   - Setting up multi-peer download
   - Configuring obfuscation
   - Running as daemon
   - Troubleshooting common issues

2. **docs/INTEGRATION_GUIDE.md** (~800 lines)
   - Embedding WRAITH in applications
   - Using the Node API
   - Custom transport implementations
   - Event handling
   - Error handling patterns

3. **docs/TROUBLESHOOTING.md** (~600 lines)
   - Connection failures (solutions for each)
   - Performance issues
   - NAT traversal problems
   - Debugging with logs
   - Common misconfigurations

4. **docs/COMPARISON.md** (~500 lines)
   - WRAITH vs WireGuard
   - WRAITH vs Signal Protocol
   - WRAITH vs Tor
   - WRAITH vs BitTorrent
   - Use case recommendations

**Tasks:**
- [ ] Write TUTORIAL.md with screenshots
- [ ] Write INTEGRATION_GUIDE.md with code examples
- [ ] Write TROUBLESHOOTING.md with solutions
- [ ] Write COMPARISON.md with benchmarks
- [ ] Update README with v1.0.0 status
- [ ] Review all docs for accuracy

**Acceptance Criteria:**
- [ ] Tutorial walks through complete workflow
- [ ] Integration guide has working code examples
- [ ] Troubleshooting covers 20+ scenarios
- [ ] Comparison is fair and accurate
- [ ] All links in docs work
- [ ] cargo doc generates clean documentation

---

### 10.4.2: Security Validation (5 SP)

**Objective:** External security audit OR comprehensive penetration testing + DPI evasion validation.

**Option A: External Security Audit (Ideal)**

Engage security firm to audit:
- Cryptographic implementation
- Memory safety
- Side-channel resistance
- Protocol security properties
- Input validation

**Cost:** $5,000 - $15,000
**Duration:** 2 weeks
**Deliverable:** Security audit report

**Option B: DIY Penetration Testing (If budget unavailable)**

```bash
#!/bin/bash
# scripts/security_validation.sh

echo "=== WRAITH Protocol Security Validation ==="

# 1. Fuzzing (continuous for 72 hours)
echo "1. Running fuzzing (72 hours)..."
cargo fuzz run frame_parser -- -max_total_time=259200 &
cargo fuzz run dht_message -- -max_total_time=259200 &
cargo fuzz run crypto -- -max_total_time=259200 &
wait

# 2. Penetration testing
echo "2. Running penetration tests..."
python3 scripts/pentest.py

# 3. DPI evasion testing
echo "3. Testing DPI evasion..."
# Capture traffic with Wireshark
sudo tcpdump -i eth0 -w /tmp/wraith_traffic.pcap &
TCPDUMP_PID=$!

# Transfer file
./target/release/wraith send test.zip --to peer-id

sleep 5
sudo kill $TCPDUMP_PID

# Analyze with Suricata
suricata -c /etc/suricata/suricata.yaml -r /tmp/wraith_traffic.pcap

# Analyze with nDPI
ndpiReader -i /tmp/wraith_traffic.pcap

# 4. Side-channel testing
echo "4. Testing timing side-channels..."
cargo test --release -- --ignored timing_tests

echo "Security validation complete."
```

**DPI Evasion Validation:**
```bash
# Test with multiple DPI tools
for tool in suricata zeek snort ndpi; do
    echo "Testing with $tool..."

    # Capture traffic
    capture_traffic

    # Analyze
    analyze_with_tool $tool

    # Verify WRAITH not detected
    if detected_as_wraith; then
        echo "FAIL: $tool detected WRAITH"
        exit 1
    fi
done

echo "PASS: All DPI tools report unknown protocol"
```

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

### 10.4.3: Reference Client Application (4 SP)

**Objective:** Create simple GUI application demonstrating WRAITH Protocol usage.

**WRAITH-Transfer (Basic GUI):**

```rust
// examples/wraith-transfer-gui/src/main.rs

use iced::{Application, Command, Element, Settings};
use wraith_core::Node;

struct TransferApp {
    node: Option<Node>,
    status: String,
    file_path: String,
    peer_id: String,
}

#[derive(Debug, Clone)]
enum Message {
    InitNode,
    NodeInitialized(Result<Node, String>),
    SendFile,
    FilePathChanged(String),
    PeerIdChanged(String),
    TransferComplete,
}

impl Application for TransferApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                node: None,
                status: "Not connected".to_string(),
                file_path: String::new(),
                peer_id: String::new(),
            },
            Command::perform(Node::new_random(), |r| {
                Message::NodeInitialized(r.map_err(|e| e.to_string()))
            }),
        )
    }

    fn title(&self) -> String {
        "WRAITH Transfer".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::NodeInitialized(Ok(node)) => {
                self.status = format!("Connected: {}", node.node_id());
                self.node = Some(node);
                Command::none()
            }
            Message::SendFile => {
                if let Some(node) = &self.node {
                    let file = self.file_path.clone();
                    let peer = self.peer_id.clone();

                    Command::perform(
                        async move {
                            node.send_file(&file, &peer.parse().unwrap()).await
                        },
                        |_| Message::TransferComplete,
                    )
                } else {
                    Command::none()
                }
            }
            // ... other messages ...
            _ => Command::none(),
        }
    }

    fn view(&mut self) -> Element<Message> {
        // Simple GUI with file picker, peer ID input, send button
        // ... implementation ...
    }
}

fn main() -> iced::Result {
    TransferApp::run(Settings::default())
}
```

**Features:**
- Drag-and-drop file selection
- Peer ID input
- Progress bar
- Status display
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

## Definition of Done (Phase 10)

### Functionality
- [ ] All v0.9.0 features working
- [ ] XDP implemented OR documented
- [ ] Rate limiting functional
- [ ] DoS protection functional
- [ ] Health monitoring functional
- [ ] Resume works under all failure modes
- [ ] Connection migration stress tested
- [ ] Multi-peer optimization complete

### Security
- [ ] Security audit passed OR penetration testing complete
- [ ] DPI evasion validated with real tools
- [ ] Fuzzing: 72 hours, zero crashes
- [ ] Side-channels tested
- [ ] Zero security vulnerabilities

### Documentation
- [ ] Tutorial complete
- [ ] Integration guide complete
- [ ] Troubleshooting guide complete
- [ ] Comparison guide complete
- [ ] All documentation reviewed
- [ ] cargo doc generates clean docs

### Testing
- [ ] All tests passing (target: 1050+ tests)
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
- [ ] v1.0.0 tag created
- [ ] Release notes written
- [ ] Binaries published
- [ ] Documentation published
- [ ] Announcement prepared

---

## Success Metrics

### Technical Metrics
- [ ] XDP status: Implemented OR Documented
- [ ] Test count: >1050 (current: 973)
- [ ] Security: External audit passed OR pentest clean
- [ ] DPI evasion: Not detected by 4+ tools

### Functional Metrics
- [ ] Resume success rate: 100% (all 5 scenarios)
- [ ] Migration success rate: 100% (all 5 scenarios)
- [ ] Multi-peer optimization: Measurable improvement

### Quality Metrics
- [ ] Zero TODOs in codebase
- [ ] Documentation: 100% complete
- [ ] Technical debt ratio: <15%
- [ ] Grade: A+ maintained

---

## Risk Management

### High-Risk Areas

**1. External Security Audit**
- **Risk:** May find critical vulnerabilities
- **Mitigation:** Thorough internal review first
- **Contingency:** Fix issues, delay release if needed

**2. DPI Evasion Validation**
- **Risk:** May be detected by some tools
- **Mitigation:** Test with multiple tools, iterate
- **Contingency:** Document known limitations

**3. Reference Client**
- **Risk:** GUI complexity may delay release
- **Mitigation:** Keep GUI minimal, CLI as fallback
- **Contingency:** Ship v1.0.0 without GUI if needed

---

## Completion Checklist

- [ ] Sprint 10.1: XDP & Performance (34 SP)
- [ ] Sprint 10.2: Production Hardening (21 SP)
- [ ] Sprint 10.3: Advanced Features (21 SP)
- [ ] Sprint 10.4: Documentation & Release (17 SP)
- [ ] All acceptance criteria met
- [ ] All documentation complete
- [ ] Security validation passed
- [ ] README updated (v1.0.0 status)
- [ ] CHANGELOG.md updated
- [ ] Release v1.0.0 prepared
- [ ] Announcement written
- [ ] Binaries published

**Estimated Completion:** 4-5 weeks

---

**WRAITH Protocol v1.0.0 - PRODUCTION COMPLETE!**

After Phase 10, WRAITH Protocol will be **100% complete**, production-hardened, comprehensively documented, and ready for public release. All protocol features implemented, all documentation complete, all tests passing, zero TODOs remaining.

**Total Project:** 947 SP (original) + 85 SP (Phase 9) + 93 SP (Phase 10) = **1,125 SP delivered**

**Public Release Ready:** ✅
