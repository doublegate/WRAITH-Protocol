# WRAITH Protocol Performance Architecture

**Document Version:** 1.0.0
**Last Updated:** 2025-11-28
**Status:** Architecture Specification

---

## Performance Targets

### Throughput Goals

| Network | Target | Theoretical Max | Actual (Expected) |
|---------|--------|-----------------|-------------------|
| **Gigabit Ethernet** | 300-900 Mbps | 1000 Mbps | 800 Mbps |
| **10 Gigabit** | 3-9 Gbps | 10 Gbps | 9.2 Gbps |
| **40 Gigabit** | 10-35 Gbps | 40 Gbps | 32 Gbps |
| **WiFi 6** | 100-400 Mbps | 600 Mbps | 350 Mbps |
| **LTE** | 10-50 Mbps | 100 Mbps | 45 Mbps |
| **5G** | 50-500 Mbps | 1000 Mbps | 400 Mbps |

### Latency Goals

| Metric | Target | Achievable |
|--------|--------|------------|
| **NIC → Userspace** | <1 μs | <500 ns (AF_XDP) |
| **Crypto Processing** | <10 μs/packet | ~5 μs (ChaCha20) |
| **Handshake RTT** | <50 ms (LAN) | 2-5 ms typical |
| **First Byte Latency** | <100 ms | 10-20 ms (LAN) |

---

## Kernel Bypass Architecture

### AF_XDP Performance Model

**Zero-Copy Path:**
```
NIC DMA → UMEM → Userspace Processing
   ↓
No kernel memcpy
No syscalls in hot path
```

**Throughput Analysis:**
```
Theoretical Maximum:
  Link Speed: 10 Gbps
  Packet Size: 1500 bytes
  Max PPS: 10e9 / (1500 * 8) ≈ 833,333 pps

AF_XDP Measured:
  Single Core: 24M pps (redirect)
  Batch Size 64: ~600K pps @ 1500 byte packets ≈ 7.2 Gbps
  Efficiency: 72% of line rate (cryptographic overhead)
```

**Latency Breakdown:**
```
Component                  Latency
────────────────────────────────────
NIC DMA to UMEM           ~100 ns
XDP program execution     ~50 ns
AF_XDP ring notification  ~100 ns
Userspace poll            ~200 ns
Total (NIC → userspace):  ~450 ns
```

### XDP/eBPF Packet Filtering

**Performance Characteristics:**
```rust
// XDP program budget: 100 instructions max (verifier limit)
// Measured performance:

Drop rate:     26M pps per core
Redirect rate: 24M pps per core
Processing:    ~40 ns per packet
```

**Optimization Techniques:**
1. **Early Return:** Drop unknown packets immediately
2. **Map Lookups:** O(1) hash table for CID → queue
3. **Minimal Parsing:** Only parse up to CID (first 8 bytes of UDP payload)
4. **No Loops:** Unrolled parsing for verifier

**Example Performance:**
```c
// Optimized XDP program
SEC("xdp")
int wraith_filter(struct xdp_md *ctx) {
    // Fast path: 4 instructions
    __u8 *cid = extract_cid_fast(ctx);

    // Hash lookup: 1 instruction
    __u32 *queue = bpf_map_lookup_elem(&conn_map, cid);

    if (queue) {
        // Redirect: 1 instruction
        return bpf_redirect_map(&xsk_map, *queue, 0);
    }

    return XDP_PASS;  // 7 total instructions for hot path
}
```

### io_uring File I/O

**Submission Queue (SQ) Batching:**
```rust
// Submit 64 read operations in one syscall
for chunk in chunks.take(64) {
    let sqe = opcode::Read::new(...)
        .build();
    ring.submission().push(&sqe)?;
}

ring.submit()?;  // Single syscall for 64 ops
```

**Performance:**
```
Operation              Traditional     io_uring
──────────────────────────────────────────────────
Read (256KB)           ~50 μs          ~30 μs
Write (256KB)          ~80 μs          ~45 μs
Fsync                  ~500 μs         ~400 μs
Batch 64 reads         3200 μs         ~150 μs
```

**Throughput:**
```
Sequential Read:  ~8 GB/s (NVMe SSD)
Random Read:      ~500 MB/s (4KB blocks)
Sequential Write: ~6 GB/s
Batch Factor:     ~20x improvement over sync I/O
```

---

## Threading Model

### Thread-per-Core Design

**Architecture:**
```
Core 0          Core 1          Core 2          Core 3
──────────────  ──────────────  ──────────────  ──────────────
IO Worker       IO Worker       IO Worker       IO Worker
AF_XDP Socket   AF_XDP Socket   AF_XDP Socket   AF_XDP Socket
Sessions[0-N]   Sessions[0-N]   Sessions[0-N]   Sessions[0-N]
io_uring        io_uring        io_uring        io_uring
NUMA Node 0     NUMA Node 0     NUMA Node 1     NUMA Node 1
```

**Benefits:**
- **No Locks:** Each core owns its data
- **Cache Locality:** Hot data in L1/L2 cache
- **No Context Switching:** Threads pinned to cores
- **Predictable Performance:** No contention

**Load Balancing:**
```rust
// Connection assignment via hash
let core_id = hash(connection_id) % num_cores;

// XDP program redirects to correct queue
return bpf_redirect_map(&xsk_map, core_id, 0);
```

### CPU Pinning

**Implementation:**
```rust
use nix::sched::{sched_setaffinity, CpuSet};

fn pin_to_core(core: usize) {
    let mut cpuset = CpuSet::new();
    cpuset.set(core).unwrap();
    sched_setaffinity(Pid::from_raw(0), &cpuset).unwrap();
}

// Worker thread
std::thread::spawn(move || {
    pin_to_core(core_id);
    worker_loop();
});
```

**NUMA Awareness:**
```rust
// Allocate memory on local NUMA node
let numa_node = get_core_numa_node(core_id);
let umem = Umem::allocate_numa(size, numa_node)?;
```

---

## Memory Architecture

### UMEM Configuration

**Size Calculation:**
```
Recommended UMEM Size:
  Frame Size: 4096 bytes
  Num Frames: 16384
  Total: 64 MB per core

Large Deployments:
  Frame Size: 4096 bytes
  Num Frames: 32768
  Total: 128 MB per core
```

**Huge Pages:**
```bash
# Enable 2MB huge pages
echo 64 > /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages

# Transparent Huge Pages (THP)
echo always > /sys/kernel/mm/transparent_hugepage/enabled
```

**Benefits:**
```
TLB Entries:
  4KB pages: 64 MB = 16,384 entries → multiple TLB misses
  2MB pages: 64 MB = 32 entries → fits in TLB

Performance Impact:
  Without huge pages: ~5-10% TLB miss overhead
  With huge pages:    <1% TLB miss overhead
```

### Memory Pools

**Packet Buffer Pool:**
```rust
struct PacketPool {
    buffers: Vec<Vec<u8>>,  // Pre-allocated packets
    free_list: VecDeque<usize>,
    size: usize,            // Packets in pool
}

impl PacketPool {
    fn new(size: usize, packet_size: usize) -> Self {
        let buffers: Vec<_> = (0..size)
            .map(|_| vec![0u8; packet_size])
            .collect();

        let free_list = (0..size).collect();

        Self { buffers, free_list, size }
    }

    fn alloc(&mut self) -> Option<&mut Vec<u8>> {
        let idx = self.free_list.pop_front()?;
        Some(&mut self.buffers[idx])
    }

    fn free(&mut self, idx: usize) {
        self.free_list.push_back(idx);
    }
}
```

**Performance:**
```
Allocation:
  malloc():         ~200 ns
  Pool allocation:  ~10 ns (pointer lookup)
  Speedup:          20x

Fragmentation:
  malloc():         High (over time)
  Pool:             Zero (fixed-size allocations)
```

### Cache Optimization

**Data Structure Alignment:**
```rust
#[repr(align(64))]  // Cache line size
struct SessionState {
    // Hot path fields (frequently accessed)
    send_window: u64,
    recv_window: u64,
    next_seq: u32,
    largest_acked: u32,

    // Padding to next cache line
    _pad: [u8; 32],

    // Cold path fields (infrequently accessed)
    created_at: Instant,
    peer_addr: SocketAddr,
    // ...
}
```

**False Sharing Prevention:**
```rust
// Per-core counters (separate cache lines)
#[repr(align(64))]
struct CoreStats {
    packets_sent: AtomicU64,
    packets_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
}

// Array of per-core stats (no false sharing)
let stats: Vec<CoreStats> = (0..num_cores)
    .map(|_| CoreStats::default())
    .collect();
```

---

## Cryptographic Performance

### ChaCha20-Poly1305 Optimization

**SIMD Vectorization:**
```rust
// x86_64 AVX2 implementation
#[cfg(target_arch = "x86_64")]
fn chacha20_avx2(key: &[u8; 32], nonce: &[u8; 24], data: &mut [u8]) {
    // Process 4 blocks (256 bytes) in parallel
    // Throughput: ~3.5 GB/s per core
}
```

**Throughput:**
```
Platform              Throughput (single core)
─────────────────────────────────────────────
x86_64 (AVX2)         ~3.5 GB/s
x86_64 (no SIMD)      ~1.2 GB/s
aarch64 (NEON)        ~2.8 GB/s
aarch64 (no SIMD)     ~900 MB/s
```

**Latency:**
```
Packet Size    Encryption Time
───────────────────────────────
64 bytes       ~200 ns
256 bytes      ~500 ns
1500 bytes     ~2.5 μs
4096 bytes     ~6 μs
```

**Optimization Techniques:**
1. **Pipelined Processing:** Interleave encryption and I/O
2. **Batch Mode:** Encrypt multiple packets together
3. **Key Caching:** Derive nonce salts once per session
4. **Assembly Intrinsics:** Use vendor-optimized implementations

### BLAKE3 Hashing

**Performance:**
```
Throughput:
  Single Thread:  ~1.5 GB/s (sequential)
  4 Threads:      ~5.5 GB/s (parallel)
  8 Threads:      ~10 GB/s (parallel)

Latency:
  256 KB chunk:   ~170 μs (single core)
  1 MB chunk:     ~680 μs (single core)
```

**Tree Hashing:**
```rust
// Parallel hashing of file chunks
use rayon::prelude::*;

let chunk_hashes: Vec<[u8; 32]> = chunks
    .par_iter()
    .map(|chunk| BLAKE3::hash(chunk).into())
    .collect();

// Combine into root hash
let root = BLAKE3::hash_tree(&chunk_hashes);
```

---

## Network Optimizations

### BBR Congestion Control

**Throughput vs. Loss Rate:**
```
Packet Loss    Traditional TCP    BBR
────────────────────────────────────────
0%             950 Mbps           980 Mbps
0.1%           700 Mbps           950 Mbps
1%             300 Mbps           850 Mbps
5%             80 Mbps            600 Mbps
```

**RTT Adaptation:**
```
RTT            Traditional    BBR
─────────────────────────────────
10 ms          980 Mbps       980 Mbps
50 ms          850 Mbps       970 Mbps
100 ms         600 Mbps       950 Mbps
200 ms         300 Mbps       900 Mbps
```

**Benefits:**
- **High BDP Networks:** Better utilization
- **Lossy Networks:** Robust to packet loss
- **Variable RTT:** Adapts quickly

### Pacing Implementation

**Token Bucket:**
```rust
struct Pacer {
    rate: u64,           // bytes/sec
    tokens: f64,         // accumulated credit
    last_update: Instant,
    max_burst: u64,      // bytes
}

impl Pacer {
    fn try_send(&mut self, packet_size: usize) -> Option<Duration> {
        // Accumulate tokens
        let elapsed = self.last_update.elapsed();
        self.tokens += elapsed.as_secs_f64() * self.rate as f64;
        self.tokens = self.tokens.min(self.max_burst as f64);
        self.last_update = Instant::now();

        // Check if enough tokens
        if self.tokens >= packet_size as f64 {
            self.tokens -= packet_size as f64;
            Some(Duration::ZERO)  // Send now
        } else {
            // Calculate wait time
            let needed = packet_size as f64 - self.tokens;
            let wait = Duration::from_secs_f64(needed / self.rate as f64);
            Some(wait)
        }
    }
}
```

**Performance Impact:**
```
Without Pacing:
  Bursts: 10-100 packets at once
  Buffer bloat: High queuing delay
  Loss: Increased at bottleneck

With Pacing:
  Smooth transmission
  Low queuing delay
  Minimal loss
  Throughput: 0-2% reduction (acceptable tradeoff)
```

### Jumbo Frames

**Configuration:**
```bash
# Enable 9000-byte MTU
ip link set eth0 mtu 9000
```

**Performance Gain:**
```
Packet Size    PPS Required    CPU Overhead
──────────────────────────────────────────
1500 bytes     833K pps        100%
9000 bytes     139K pps        20%

Throughput (10 Gbps):
  1500-byte:   85% efficient (crypto overhead)
  9000-byte:   95% efficient (lower overhead)
```

**Caveats:**
- Requires jumbo frame support on all hops
- Internet generally limited to 1500 MTU
- Use PLPMTUD for discovery

---

## Benchmarking

### Throughput Benchmark

**Test Setup:**
```rust
// Send 1 GB file, measure throughput
let file_size = 1_073_741_824;  // 1 GB
let start = Instant::now();

let transfer_id = wraith.send_file("test.bin", peer)?;
wraith.wait_completion(transfer_id)?;

let elapsed = start.elapsed();
let throughput = file_size as f64 / elapsed.as_secs_f64();

println!("Throughput: {:.2} Mbps", throughput * 8.0 / 1_000_000.0);
```

**Expected Results:**
```
Hardware: Intel i7-12700K, 10GbE NIC
Network: Direct cable, <1ms RTT

File Size     Throughput     CPU Usage
─────────────────────────────────────
1 MB          950 Mbps       15%
10 MB         9.2 Gbps       45%
100 MB        9.5 Gbps       50%
1 GB          9.5 Gbps       50%
10 GB         9.5 Gbps       50%
```

### Latency Benchmark

**Ping-Pong Test:**
```rust
// Measure round-trip time
let mut samples = Vec::new();

for _ in 0..1000 {
    let start = Instant::now();

    wraith.send_ping(peer)?;
    let pong = wraith.recv_pong()?;

    let rtt = start.elapsed();
    samples.push(rtt);
}

let median = percentile(&samples, 50.0);
let p99 = percentile(&samples, 99.0);

println!("Median RTT: {:?}", median);
println!("P99 RTT: {:?}", p99);
```

**Expected Results:**
```
Network      Median RTT    P99 RTT
────────────────────────────────────
LAN          2 ms          5 ms
WAN (100km)  15 ms         25 ms
Intercont.   150 ms        200 ms
```

### CPU Profiling

**Tools:**
```bash
# perf profiling
perf record -g ./wraith-cli send large-file.bin
perf report

# flamegraph
cargo flamegraph --bin wraith-cli -- send large-file.bin
```

**Expected Hotspots:**
```
Function                  % CPU Time
────────────────────────────────────
chacha20_encrypt          35%
blake3_hash               15%
af_xdp_recv_batch         12%
frame_parse               8%
bbr_update_model          6%
session_process_frame     5%
Other                     19%
```

---

## Performance Tuning

### Kernel Parameters

**Network Stack:**
```bash
# Increase buffer sizes
sysctl -w net.core.rmem_max=268435456   # 256 MB
sysctl -w net.core.wmem_max=268435456
sysctl -w net.core.rmem_default=16777216 # 16 MB
sysctl -w net.core.wmem_default=16777216

# Increase connection table
sysctl -w net.core.netdev_max_backlog=30000
sysctl -w net.ipv4.tcp_max_syn_backlog=8192

# Disable TCP timestamps (reduce overhead)
sysctl -w net.ipv4.tcp_timestamps=0
```

**AF_XDP:**
```bash
# Enable busy polling
sysctl -w net.core.busy_poll=50
sysctl -w net.core.busy_read=50

# XDP native mode (requires driver support)
ethtool -K eth0 xdp-native on
```

**CPU:**
```bash
# Disable CPU frequency scaling
cpupower frequency-set -g performance

# Disable hyper-threading (optional, for consistency)
echo off > /sys/devices/system/cpu/smt/control

# Set interrupt affinity
echo 1 > /proc/irq/<IRQ>/smp_affinity
```

### Application Configuration

**WRAITH Config:**
```toml
[performance]
# Worker threads (1 per core recommended)
workers = 8

# AF_XDP configuration
xdp_mode = "native"  # native, skb, or disabled
umem_size_mb = 64
frame_size = 4096
ring_size = 4096

# Batching
rx_batch_size = 64
tx_batch_size = 64

# Memory
use_huge_pages = true
numa_aware = true

# Crypto
simd = "avx2"  # avx2, neon, or auto

[obfuscation]
# Disable for maximum performance
mode = "performance"  # performance, privacy, stealth
cover_traffic = false
timing_obfuscation = false
```

---

## Performance Troubleshooting

### Common Bottlenecks

**1. CPU Saturation**
```
Symptom: Throughput plateaus, CPU at 100%
Diagnosis:
  - perf top → identify hot functions
  - Check crypto overhead (should be <50% CPU)
Solution:
  - Reduce encryption strength (not recommended)
  - Add more cores
  - Optimize hot paths
```

**2. Memory Bandwidth**
```
Symptom: Throughput < expected, low CPU utilization
Diagnosis:
  - perf stat -e cache-misses
  - Monitor NUMA node traffic
Solution:
  - Enable huge pages
  - Ensure NUMA-local allocation
  - Increase buffer pool sizes
```

**3. Network Interface**
```
Symptom: Packet drops, retransmissions
Diagnosis:
  - ethtool -S eth0 | grep drop
  - Check XDP statistics
Solution:
  - Increase ring buffer sizes (ethtool -G)
  - Enable RSS (Receive Side Scaling)
  - Upgrade NIC firmware
```

**4. Disk I/O**
```
Symptom: File transfer slower than network
Diagnosis:
  - iostat -x 1 → check %util
  - iotop → identify process
Solution:
  - Use faster storage (NVMe)
  - Increase io_uring queue depth
  - Enable write-back caching (if safe)
```

### Monitoring

**Key Metrics:**
```rust
pub struct Metrics {
    // Throughput
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,

    // Packet stats
    packets_sent: AtomicU64,
    packets_received: AtomicU64,
    packets_dropped: AtomicU64,

    // Latency
    rtt_samples: Histogram,
    processing_time: Histogram,

    // Errors
    crypto_errors: AtomicU64,
    io_errors: AtomicU64,
    network_errors: AtomicU64,

    // Resource usage
    umem_usage: AtomicUsize,
    session_count: AtomicUsize,
    stream_count: AtomicUsize,
}
```

**Alerting Thresholds:**
```
Metric                   Warning    Critical
─────────────────────────────────────────────
Packet Drop Rate         >1%        >5%
Crypto Error Rate        >0.1%      >1%
CPU Usage                >80%       >95%
UMEM Usage               >80%       >95%
Median RTT Increase      >2x        >5x
```

---

## Conclusion

WRAITH's performance architecture achieves wire-speed throughput through:
- Kernel bypass (AF_XDP) for zero-copy packet I/O
- Thread-per-core design eliminating contention
- NUMA-aware memory allocation
- Optimized cryptographic implementations (SIMD)
- BBR congestion control for high BDP networks
- Batched operations throughout the stack

**Expected Performance:**
- 10 Gbps: 9+ Gbps sustained
- Sub-millisecond latency (NIC to application)
- <50% CPU utilization at line rate

---

**See Also:**
- [Protocol Overview](protocol-overview.md)
- [Layer Design](layer-design.md)
- [Performance Tuning](../operations/performance-tuning.md)
