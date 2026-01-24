# WRAITH Protocol v2 Performance Targets

**Version:** 1.0.0
**Date:** 2026-01-24
**Status:** Planning Document
**Authors:** WRAITH Protocol Team

---

## Table of Contents

1. [Overview](#overview)
2. [Throughput Targets](#throughput-targets)
3. [Latency Targets](#latency-targets)
4. [Resource Utilization](#resource-utilization)
5. [Scalability Targets](#scalability-targets)
6. [Benchmark Methodology](#benchmark-methodology)
7. [Performance Comparison](#performance-comparison)
8. [Optimization Strategies](#optimization-strategies)

---

## Overview

This document defines performance targets for WRAITH Protocol v2, establishing measurable goals for throughput, latency, resource utilization, and scalability across different deployment scenarios.

### Performance Philosophy

1. **Correctness First:** Security and correctness never sacrificed for performance
2. **Measurable Goals:** All targets have specific, measurable criteria
3. **Realistic Baselines:** Targets based on hardware capabilities and protocol overhead
4. **Graceful Degradation:** Performance degrades predictably under load

### Target Environments

| Environment | Description | Primary Metric |
|-------------|-------------|----------------|
| Embedded | ARM Cortex-A, 256MB RAM | Resource efficiency |
| Mobile | Android/iOS, battery constraints | Power efficiency |
| Desktop | Modern x86-64, 8GB+ RAM | Throughput |
| Server | High-core count, 64GB+ RAM | Scalability |
| High-Performance | DPDK/AF_XDP, 100GbE | Wire-speed |

---

## Throughput Targets

### Network Throughput

| Transport Mode | Target | Conditions |
|----------------|--------|------------|
| Userspace UDP | 500 Mbps | Single core, 8KB MTU |
| Userspace UDP | 2 Gbps | Multi-core (4), 8KB MTU |
| io_uring | 5 Gbps | Multi-core (4), 8KB MTU |
| AF_XDP | 25 Gbps | Single core, dedicated NIC |
| AF_XDP | 100 Gbps | Multi-core (8), multi-queue NIC |

### Throughput by Packet Size

```
Throughput vs Packet Size (AF_XDP, single core):
═══════════════════════════════════════════════

     Throughput (Gbps)
     │
 100 ┤                                    ┌───────
     │                               ┌────┘
  75 ┤                          ┌────┘
     │                     ┌────┘
  50 ┤                ┌────┘
     │           ┌────┘
  25 ┤      ┌────┘
     │ ┌────┘
   0 ┼─────┴─────┴─────┴─────┴─────┴─────┴─────►
         64   256   512  1024  1500  4096  9000
                     Packet Size (bytes)

Performance Notes:
- 64B packets: ~15 Mpps (line rate limited)
- 1500B packets: ~8.3 Mpps (~100 Gbps)
- 9000B jumbo: ~1.4 Mpps (~100 Gbps)
```

### Crypto Throughput

| Operation | Target | Algorithm |
|-----------|--------|-----------|
| AEAD Encrypt | 10 GB/s | XChaCha20-Poly1305 |
| AEAD Decrypt | 10 GB/s | XChaCha20-Poly1305 |
| Key Exchange (Classical) | 50,000 ops/s | X25519 |
| Key Exchange (Hybrid) | 10,000 ops/s | X25519 + ML-KEM-768 |
| Signature (Classical) | 25,000 ops/s | Ed25519 |
| Signature (Hybrid) | 5,000 ops/s | Ed25519 + ML-DSA-65 |
| Hash | 15 GB/s | BLAKE3 (SIMD) |
| Ratchet Advance | 5,000,000 ops/s | BLAKE3 KDF |

### File Transfer Throughput

| Operation | Target | Notes |
|-----------|--------|-------|
| Chunking | 15 GB/s | BLAKE3 tree hashing |
| Reassembly | 10 GB/s | io_uring writes |
| Delta Transfer | 5 GB/s | rsync-style chunking |
| Encryption | 8 GB/s | With AEAD |

---

## Latency Targets

### Handshake Latency

| Phase | Target | Components |
|-------|--------|------------|
| Version Negotiation | < 1 RTT | Version probe + response |
| Key Exchange (Classical) | < 2 RTT | Noise_XX pattern |
| Key Exchange (Hybrid) | < 3 RTT | Noise_XX + ML-KEM |
| Session Establishment | < 50 ms | Total (LAN) |
| Session Establishment | < 200 ms | Total (WAN, 100ms RTT) |

### Packet Latency

```
Packet Processing Latency Breakdown:
════════════════════════════════════

Component                   Target      Notes
─────────────────────────────────────────────────
Receive from NIC            < 1 μs      AF_XDP zero-copy
Header Parsing              < 100 ns    Polymorphic decode
Crypto Verify (AEAD)        < 500 ns    Per packet
Ratchet Advance             < 50 ns     Per packet
Payload Processing          < 100 ns    Copy to buffer
Application Delivery        < 200 ns    Queue to app
─────────────────────────────────────────────────
Total (Hot Path)            < 2 μs      End-to-end

Additional Latency:
- Session Lookup            < 100 ns    Hash table
- Congestion Control        < 50 ns     BBRv3 update
- ACK Generation            < 100 ns    Delayed ACK batch
```

### End-to-End Latency

| Scenario | Target | Network Conditions |
|----------|--------|-------------------|
| LAN (1 Gbps) | < 1 ms | < 1ms network RTT |
| WAN (100ms RTT) | < 110 ms | 100ms network RTT |
| Mobile (4G) | < 150 ms | ~50ms base RTT |
| Satellite | < 700 ms | ~600ms base RTT |

### Tail Latency

| Percentile | Target | Notes |
|------------|--------|-------|
| P50 | < 2 μs | Median packet |
| P95 | < 10 μs | Most packets |
| P99 | < 50 μs | Allow rare slowdowns |
| P99.9 | < 500 μs | Outliers |

---

## Resource Utilization

### Memory Targets

| Component | Target (per session) | Notes |
|-----------|---------------------|-------|
| Session State | < 2 KB | Core metadata |
| Crypto State | < 1 KB | Keys, ratchet |
| Buffers | < 64 KB | Configurable |
| Total | < 100 KB | Default config |

| Scale | Target | Notes |
|-------|--------|-------|
| 1,000 sessions | < 100 MB | Desktop use case |
| 10,000 sessions | < 1 GB | Server use case |
| 100,000 sessions | < 10 GB | High-scale server |
| 1,000,000 sessions | < 100 GB | Extreme scale |

### CPU Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Packets per core | 5M pps | AF_XDP |
| Sessions per core | 10,000 | Active |
| Handshakes per core | 1,000/s | Hybrid |
| Idle CPU (per session) | < 0.001% | When quiet |

### Power Targets (Mobile)

| State | Target | Notes |
|-------|--------|-------|
| Idle (connected) | < 5 mW | Keep-alive mode |
| Low Activity | < 50 mW | Occasional packets |
| Active Transfer | < 500 mW | Full throughput |
| Handshake | < 1 W | Peak during PQ crypto |

---

## Scalability Targets

### Connection Scaling

```
Connection Scaling Curve:
═════════════════════════

Sessions     Throughput      CPU Usage    Memory
────────────────────────────────────────────────
100          500 Mbps        5%           10 MB
1,000        2 Gbps          20%          100 MB
10,000       5 Gbps          50%          1 GB
100,000      10 Gbps         80%          10 GB
1,000,000    10 Gbps         95%          100 GB

Note: Throughput limited by NIC, not protocol
```

### Horizontal Scaling

| Metric | Target | Notes |
|--------|--------|-------|
| Nodes | 1,000+ | DHT-based discovery |
| Linear scaling | 90% efficiency | Up to 100 nodes |
| Geo-distribution | Global | Multi-region |

### Vertical Scaling

| Cores | Expected Throughput | Notes |
|-------|---------------------|-------|
| 1 | 25 Gbps | AF_XDP baseline |
| 4 | 90 Gbps | 90% linear |
| 8 | 100 Gbps | NIC limited |
| 16 | 100 Gbps | NIC limited |

---

## Benchmark Methodology

### Standard Benchmarks

```rust
/// Performance benchmark suite
pub mod benchmarks {
    use criterion::{criterion_group, criterion_main, Criterion, Throughput};

    /// Handshake latency benchmark
    pub fn bench_handshake(c: &mut Criterion) {
        let mut group = c.benchmark_group("handshake");

        // Classical-only handshake
        group.bench_function("classical", |b| {
            b.iter(|| {
                let (client, server) = create_test_pair();
                complete_classical_handshake(client, server)
            })
        });

        // Hybrid handshake
        group.bench_function("hybrid", |b| {
            b.iter(|| {
                let (client, server) = create_test_pair();
                complete_hybrid_handshake(client, server)
            })
        });

        group.finish();
    }

    /// Throughput benchmark
    pub fn bench_throughput(c: &mut Criterion) {
        let mut group = c.benchmark_group("throughput");

        for size in [1024, 8192, 65536, 1048576].iter() {
            group.throughput(Throughput::Bytes(*size as u64));
            group.bench_with_input(
                format!("encrypt_{}", size),
                size,
                |b, &size| {
                    let session = create_test_session();
                    let data = vec![0u8; size];
                    b.iter(|| session.encrypt(&data))
                },
            );
        }

        group.finish();
    }

    /// Packet processing latency
    pub fn bench_packet_latency(c: &mut Criterion) {
        let session = create_test_session();
        let packet = create_test_packet(1024);

        c.bench_function("packet_process", |b| {
            b.iter(|| {
                session.process_packet(&packet)
            })
        });
    }

    criterion_group!(
        benches,
        bench_handshake,
        bench_throughput,
        bench_packet_latency,
    );
    criterion_main!(benches);
}
```

### Benchmark Configurations

| Profile | Description | Use Case |
|---------|-------------|----------|
| Quick | 100 iterations, 1 second | CI/CD |
| Standard | 1000 iterations, 10 seconds | Development |
| Extended | 10000 iterations, 60 seconds | Release validation |
| Stress | 100000 iterations, 600 seconds | Performance analysis |

### Test Environments

| Environment | Specification | Purpose |
|-------------|---------------|---------|
| CI | 4-core, 8GB RAM | Regression detection |
| Lab | 32-core, 128GB RAM, 100GbE | Peak performance |
| Cloud | c6i.xlarge (AWS) | Reproducible baseline |
| Edge | Raspberry Pi 4 | Embedded performance |

---

## Performance Comparison

### v1 vs v2 Comparison

| Metric | v1 | v2 | Improvement |
|--------|----|----|-------------|
| Handshake (classical) | 45 ms | 40 ms | 11% faster |
| Handshake (hybrid) | N/A | 55 ms | New feature |
| Max throughput (userspace) | 200 Mbps | 500 Mbps | 2.5x |
| Max throughput (AF_XDP) | 10 Gbps | 100 Gbps | 10x |
| Memory per session | 150 KB | 100 KB | 33% reduction |
| Ratchet overhead | 1 μs/min | 50 ns/packet | Different model |
| Sessions per core | 5,000 | 10,000 | 2x |

### Overhead Analysis

```
v2 Protocol Overhead:
═════════════════════

Per-Packet Overhead:
┌─────────────────────────────────────────────────────────┐
│ Component              Bytes        % of 1500B packet  │
├─────────────────────────────────────────────────────────┤
│ CID Hint                 8               0.5%          │
│ Frame Header            24               1.6%          │
│ Auth Tag                16               1.1%          │
│ Padding (avg)           32               2.1%          │
├─────────────────────────────────────────────────────────┤
│ Total Overhead          80               5.3%          │
│ Payload                1420              94.7%         │
└─────────────────────────────────────────────────────────┘

Per-Handshake Overhead (Hybrid):
┌─────────────────────────────────────────────────────────┐
│ Component              Bytes        Time (μs)          │
├─────────────────────────────────────────────────────────┤
│ X25519 exchange         64              50             │
│ ML-KEM exchange       2272             500             │
│ Noise messages        ~500             100             │
│ ML-KEM encap/decap   ~3000            1000             │
├─────────────────────────────────────────────────────────┤
│ Total                 ~6000            1650 μs         │
└─────────────────────────────────────────────────────────┘

Comparison with TLS 1.3:
┌─────────────────────────────────────────────────────────┐
│ Metric                 WRAITH v2      TLS 1.3          │
├─────────────────────────────────────────────────────────┤
│ Handshake RTT          2-3            1-2 (0-RTT)      │
│ Post-quantum           Yes            Optional         │
│ Per-packet overhead    80B            29B (GCM)        │
│ Forward secrecy        Per-packet     Per-session      │
│ Traffic analysis       Resistant      Vulnerable       │
└─────────────────────────────────────────────────────────┘
```

---

## Optimization Strategies

### CPU Optimization

```rust
/// SIMD-accelerated operations
pub mod simd_ops {
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    /// XOR buffers using AVX2
    #[cfg(target_feature = "avx2")]
    pub unsafe fn xor_buffers_avx2(a: &mut [u8], b: &[u8]) {
        assert_eq!(a.len(), b.len());

        let mut i = 0;
        while i + 32 <= a.len() {
            let va = _mm256_loadu_si256(a[i..].as_ptr() as *const __m256i);
            let vb = _mm256_loadu_si256(b[i..].as_ptr() as *const __m256i);
            let vr = _mm256_xor_si256(va, vb);
            _mm256_storeu_si256(a[i..].as_mut_ptr() as *mut __m256i, vr);
            i += 32;
        }

        // Handle remainder
        while i < a.len() {
            a[i] ^= b[i];
            i += 1;
        }
    }

    /// Batch BLAKE3 hashing
    pub fn batch_hash(inputs: &[&[u8]]) -> Vec<[u8; 32]> {
        // Use BLAKE3's parallel hashing capability
        inputs.par_iter()
            .map(|input| blake3::hash(input).into())
            .collect()
    }
}
```

### Memory Optimization

```rust
/// Memory pool for packet buffers
pub struct PacketPool {
    /// Free buffer list
    free_list: crossbeam::queue::ArrayQueue<Box<[u8; MTU]>>,

    /// Pool size
    capacity: usize,
}

impl PacketPool {
    pub fn new(capacity: usize) -> Self {
        let free_list = ArrayQueue::new(capacity);

        // Pre-allocate buffers
        for _ in 0..capacity {
            let buf = Box::new([0u8; MTU]);
            let _ = free_list.push(buf);
        }

        Self { free_list, capacity }
    }

    /// Get buffer from pool (zero-allocation fast path)
    pub fn get(&self) -> PooledBuffer {
        match self.free_list.pop() {
            Some(buf) => PooledBuffer {
                buffer: Some(buf),
                pool: self,
            },
            None => PooledBuffer {
                buffer: Some(Box::new([0u8; MTU])),
                pool: self,
            },
        }
    }
}

impl Drop for PooledBuffer<'_> {
    fn drop(&mut self) {
        if let Some(buf) = self.buffer.take() {
            let _ = self.pool.free_list.push(buf);
        }
    }
}
```

### Network Optimization

```rust
/// Batch packet transmission
pub struct BatchTransmitter {
    /// io_uring instance
    ring: IoUring,

    /// Pending submissions
    pending: Vec<SubmissionEntry>,

    /// Batch size threshold
    batch_size: usize,
}

impl BatchTransmitter {
    /// Queue packet for transmission
    pub fn queue(&mut self, packet: &[u8]) -> Result<()> {
        let sqe = opcode::Send::new(
            types::Fd(self.socket_fd),
            packet.as_ptr(),
            packet.len() as u32,
        ).build();

        self.pending.push(sqe);

        // Submit batch when threshold reached
        if self.pending.len() >= self.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    /// Flush pending transmissions
    pub fn flush(&mut self) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }

        unsafe {
            let sq = self.ring.submission();
            for sqe in self.pending.drain(..) {
                sq.push(&sqe)?;
            }
        }

        self.ring.submit_and_wait(self.pending.len())?;
        Ok(())
    }
}
```

### Congestion Control Optimization

```rust
/// BBRv3 fast path
pub struct BbrFastPath {
    /// Current sending rate
    pacing_rate: u64,

    /// Congestion window
    cwnd: u32,

    /// RTT estimate
    min_rtt: Duration,

    /// Bandwidth estimate
    btl_bw: u64,
}

impl BbrFastPath {
    /// Update on ACK (inline for hot path)
    #[inline(always)]
    pub fn on_ack(&mut self, ack: &AckInfo) {
        // Update RTT if new minimum
        if ack.rtt < self.min_rtt {
            self.min_rtt = ack.rtt;
        }

        // Update bandwidth estimate
        let delivered = ack.delivered_bytes;
        let interval = ack.delivery_time;
        let sample_bw = (delivered as u64 * 1_000_000_000) / interval.as_nanos() as u64;

        if sample_bw > self.btl_bw {
            self.btl_bw = sample_bw;
        }

        // Update pacing rate: BtlBw * gain
        self.pacing_rate = self.btl_bw * 125 / 100; // 1.25x gain
    }

    /// Bytes available to send
    #[inline(always)]
    pub fn bytes_available(&self, in_flight: u32) -> u32 {
        self.cwnd.saturating_sub(in_flight)
    }
}
```

---

## Related Documents

- [Specification](01-WRAITH-Protocol-v2-Specification.md) - Protocol specification
- [Architecture](02-WRAITH-Protocol-v2-Architecture.md) - System architecture
- [Implementation Guide](07-WRAITH-Protocol-v2-Implementation-Guide.md) - Implementation details
- [Testing Strategy](14-WRAITH-Protocol-v2-Testing-Strategy.md) - Test approach

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-24 | Initial performance targets document |
