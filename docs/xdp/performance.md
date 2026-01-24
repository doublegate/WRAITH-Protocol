# XDP Performance Guide

## Overview

This document provides performance expectations, benchmarking methodology, and optimization techniques for WRAITH's XDP implementation.

## Performance Targets

### Throughput

| Network Speed | XDP (Zero-Copy) | XDP (Copy Mode) | UDP Fallback | Speedup |
|---------------|-----------------|-----------------|--------------|---------|
| 1 Gbps | 950 Mbps | 800 Mbps | 300 Mbps | 3.2x / 2.7x |
| 10 Gbps | 9.5 Gbps | 5 Gbps | 300 Mbps | 31.7x / 16.7x |
| 25 Gbps | 24 Gbps | 10 Gbps | 300 Mbps | 80x / 33.3x |
| 40 Gbps | 38 Gbps | 15 Gbps | 300 Mbps | 126.7x / 50x |
| 100 Gbps | 95 Gbps | 20 Gbps | 300 Mbps | 316.7x / 66.7x |

**Note**: Zero-copy requires compatible NIC driver (ixgbe, i40e, ice, mlx5).

### Latency

| Metric | XDP (Zero-Copy) | XDP (Copy Mode) | UDP Fallback |
|--------|-----------------|-----------------|--------------|
| NIC → Userspace | <1 μs | 5-10 μs | 20-50 μs |
| Userspace → NIC | <1 μs | 5-10 μs | 20-50 μs |
| Round-Trip Time (RTT) | <2 μs | 10-20 μs | 40-100 μs |
| 99th percentile | <10 μs | 30-50 μs | 100-200 μs |

**Note**: Latency includes kernel-userspace handoff only, not network propagation delay.

### CPU Efficiency

| Metric | XDP (Zero-Copy) | XDP (Copy Mode) | UDP Fallback |
|--------|-----------------|-----------------|--------------|
| Cycles/packet | ~100 | ~500 | ~10,000 |
| CPU % at 1 Gbps | 5% | 10% | 80% |
| CPU % at 10 Gbps | 50% | 80% | 800% (impossible) |
| Packets/sec (single core) | 14.8 Mpps | 10 Mpps | 400 Kpps |

**Note**: Based on 2.96 GHz CPU (Intel Xeon E5-2690 v4).

## Benchmarking Methodology

### Test Environment

**Hardware**:
- CPU: Intel Xeon E5-2690 v4 (2.9 GHz, 14 cores)
- RAM: 64 GB DDR4-2400
- NIC: Intel X710 (10 GbE, i40e driver)
- Switch: 10 GbE non-blocking

**Software**:
- OS: Ubuntu 22.04 LTS (kernel 6.2.0)
- WRAITH: 0.9.0 Beta
- Rust: 1.88

### Benchmark 1: Maximum Throughput

**Objective**: Measure peak throughput with large file transfer.

**Setup**:
```bash
# Sender (10.0.0.1)
wraith send /dev/zero --to <receiver-id> --size 10GB

# Receiver (10.0.0.2)
wraith receive --output /dev/null
```

**Measurement**:
```bash
# Monitor throughput
watch -n1 'ethtool -S eth0 | grep rx_bytes'

# Calculate throughput (Mbps)
throughput = (bytes_received / test_duration_sec) * 8 / 1_000_000
```

**Expected Results**:
- XDP zero-copy: 9.5+ Gbps
- XDP copy mode: 5+ Gbps
- UDP fallback: ~300 Mbps

### Benchmark 2: Latency Under Load

**Objective**: Measure RTT latency while saturating link.

**Setup**:
```bash
# Background traffic (saturate link)
wraith send large_file.dat --to <peer-id> &

# Measure ping latency
wraith ping <peer-id> --count 1000 --interval 1ms
```

**Measurement**:
```bash
# Analyze latency distribution
wraith ping <peer-id> --count 10000 | \
  awk '{print $7}' | \
  sort -n | \
  awk '
    BEGIN {count=0; sum=0}
    {
      values[count++] = $1
      sum += $1
    }
    END {
      print "Min:", values[0]
      print "Median:", values[int(count/2)]
      print "Mean:", sum/count
      print "99th:", values[int(count*0.99)]
      print "Max:", values[count-1]
    }'
```

**Expected Results** (XDP zero-copy):
- Min: <1 μs
- Median: ~2 μs
- Mean: ~5 μs
- 99th: <10 μs
- Max: <50 μs

### Benchmark 3: CPU Utilization

**Objective**: Measure CPU overhead at different throughputs.

**Setup**:
```bash
# Start mpstat to monitor CPU usage
mpstat -P ALL 1 > cpu_stats.txt &

# Transfer at different rates
for rate in 1G 5G 10G; do
  wraith send file.dat --to <peer-id> --rate $rate
  sleep 10
done
```

**Measurement**:
```bash
# Analyze CPU usage
awk '/Average/ {print $3, $5}' cpu_stats.txt
```

**Expected Results** (10 Gbps XDP zero-copy):
- Total CPU: 50-60%
- System CPU: 5-10%
- User CPU: 45-50%

### Benchmark 4: Multi-Core Scaling

**Objective**: Verify linear scaling with multiple cores.

**Setup**:
```bash
# Configure number of queues
sudo ethtool -L eth0 combined $CORES

# Run benchmark with increasing core count
for cores in 1 2 4 8; do
  wraith send file.dat --to <peer-id> --workers $cores
done
```

**Expected Results**:
| Cores | Throughput (XDP) | Scaling Efficiency |
|-------|------------------|--------------------|
| 1 | 10 Gbps | 100% |
| 2 | 20 Gbps | 100% |
| 4 | 40 Gbps | 100% |
| 8 | 80 Gbps | 100% |

**Note**: Linear scaling assumes sufficient NIC queues and network capacity.

## Performance Optimization

### 1. CPU Affinity and Isolation

Pin worker threads to specific CPU cores for cache locality:

```bash
# Isolate CPUs 0-3 for WRAITH (kernel cmdline)
# Add to /etc/default/grub:
GRUB_CMDLINE_LINUX="isolcpus=0-3 nohz_full=0-3 rcu_nocbs=0-3"

# Update grub
sudo update-grub && sudo reboot

# Run WRAITH with pinned cores
wraith daemon --bind 0.0.0.0:40000 --cpus 0-3
```

**Impact**: +10-20% throughput, -30% latency jitter

### 2. NUMA Awareness

Allocate UMEM on same NUMA node as NIC:

```bash
# Check NIC's NUMA node
cat /sys/class/net/eth0/device/numa_node
# Output: 0

# Run WRAITH on same NUMA node
numactl --cpunodebind=0 --membind=0 wraith daemon
```

**Impact**: +15% throughput on multi-socket systems

### 3. Interrupt Tuning

Configure interrupt coalescing for batching:

```bash
# Reduce interrupt rate (higher throughput, higher latency)
sudo ethtool -C eth0 rx-usecs 50 tx-usecs 50

# Increase interrupt rate (lower latency, lower throughput)
sudo ethtool -C eth0 rx-usecs 1 tx-usecs 1

# WRAITH recommendation (balanced)
sudo ethtool -C eth0 rx-usecs 10 tx-usecs 10
```

**Impact**:
- 50 μs coalescing: +20% throughput, +25 μs latency
- 1 μs coalescing: -10% throughput, -15 μs latency

### 4. Ring Size Tuning

Larger ring buffers reduce wakeups but increase latency:

```bash
# Increase ring size (higher throughput)
wraith daemon --rx-ring-size 4096 --tx-ring-size 4096

# Decrease ring size (lower latency)
wraith daemon --rx-ring-size 512 --tx-ring-size 512

# WRAITH default (balanced)
wraith daemon --rx-ring-size 2048 --tx-ring-size 2048
```

**Impact**:
- 4096 rings: +10% throughput, +50 μs latency
- 512 rings: -5% throughput, -30 μs latency

### 5. Huge Pages

Use 2 MB huge pages for UMEM allocation:

```bash
# Allocate huge pages
echo 128 | sudo tee /proc/sys/vm/nr_hugepages

# Enable huge pages in WRAITH
wraith daemon --umem-huge-pages
```

**Impact**: +5-10% throughput, -10% CPU usage

### 6. NIC Offloads

Enable hardware offloads to reduce CPU overhead:

```bash
# Enable all offloads
sudo ethtool -K eth0 \
  rx-checksumming on \
  tx-checksumming on \
  sg on \
  tso on \
  gso on \
  gro on \
  lro on

# Verify
sudo ethtool -k eth0 | grep on
```

**Impact**: +20-30% throughput, -40% CPU usage

**Note**: Some offloads may not be compatible with XDP. Disable if XDP fails to initialize.

### 7. Batching

Process packets in batches to amortize syscall overhead:

```rust
// WRAITH default batch size: 32
let config = SocketConfig {
    batch_size: 32,
    ..Default::default()
};

// Large batches (high throughput, high latency)
batch_size: 128

// Small batches (low latency, low throughput)
batch_size: 8
```

**Impact**:
- 128 batch: +15% throughput, +100 μs latency
- 8 batch: -10% throughput, -50 μs latency

## Performance Profiling

### CPU Profiling with perf

```bash
# Profile WRAITH with perf
sudo perf record -F 99 -a -g -- wraith send file.dat --to <peer-id>

# Analyze results
sudo perf report --stdio | head -50
```

**Expected hotspots** (XDP zero-copy):
- 40-50%: Crypto (XChaCha20-Poly1305)
- 20-30%: Frame parsing and serialization
- 10-20%: AF_XDP socket operations
- 5-10%: BBR congestion control
- 5-10%: Misc (hashing, memory allocation)

### Flamegraph Visualization

```bash
# Generate flamegraph
sudo perf record -F 99 -a -g -- wraith send file.dat --to <peer-id>
sudo perf script | stackcollapse-perf.pl | flamegraph.pl > flamegraph.svg

# Open in browser
firefox flamegraph.svg
```

### Memory Profiling

```bash
# Monitor memory usage
watch -n1 'ps aux | grep wraith | grep -v grep'

# Expected memory usage:
# - Resident: 50-200 MB (depends on config)
# - Virtual: 500 MB - 2 GB (includes UMEM)
# - Shared: 10-50 MB (shared libraries)
```

### Network Profiling with bpftool

```bash
# Check XDP program (if loaded)
sudo bpftool prog show

# Check XDP map stats
sudo bpftool map show

# Dump XDP statistics
sudo bpftool prog dump xlated id <prog-id>
```

## Performance Comparison

### vs. Standard TCP/UDP

| Metric | WRAITH + XDP | TCP | UDP |
|--------|--------------|-----|-----|
| Throughput (10G) | 9.5 Gbps | 4-6 Gbps | 300 Mbps |
| Latency | <1 μs | 50-100 μs | 20-50 μs |
| CPU (10G) | 50% | 150% | 200% |
| Zero-copy | ✅ | ❌ | ❌ |
| Congestion control | BBR (custom) | CUBIC/BBR | ❌ |

### vs. DPDK

| Metric | WRAITH + XDP | DPDK |
|--------|--------------|------|
| Throughput (10G) | 9.5 Gbps | 10 Gbps |
| Throughput (100G) | 95 Gbps | 100 Gbps |
| Latency | <1 μs | <0.5 μs |
| CPU pinning | Optional | Required |
| Kernel integration | ✅ Yes | ❌ No |
| Setup complexity | ⭐⭐ | ⭐⭐⭐⭐⭐ |

**Conclusion**: XDP provides 95% of DPDK's performance with 20% of the complexity.

### vs. kernel_bypass (io_uring only)

| Metric | WRAITH + XDP | io_uring only |
|--------|--------------|---------------|
| Network I/O | Zero-copy | 2-3 copies |
| File I/O | Zero-copy | Zero-copy |
| Throughput | 9.5 Gbps | 1-2 Gbps |
| Latency | <1 μs | 20-50 μs |
| Kernel bypass | Partial | ❌ |

**Conclusion**: XDP is essential for network I/O performance; io_uring alone is insufficient.

## Troubleshooting Performance Issues

### Issue: Throughput < 50% Expected

**Diagnosis**:
```bash
# Check for packet drops
ethtool -S eth0 | grep drop

# Check CPU saturation
mpstat -P ALL 1 10

# Check interrupt distribution
cat /proc/interrupts | grep eth0
```

**Solutions**:
- Packet drops: Increase ring sizes, enable batching
- CPU saturation: Add more workers, enable CPU pinning
- Uneven interrupts: Configure IRQ affinity

### Issue: High Latency Jitter

**Diagnosis**:
```bash
# Check for context switches
pidstat -w -p $(pgrep wraith) 1 10

# Check for page faults
pidstat -r -p $(pgrep wraith) 1 10
```

**Solutions**:
- Context switches: Increase priority, isolate CPUs
- Page faults: Lock memory, enable huge pages

### Issue: Zero-Copy Not Working

**Diagnosis**:
```bash
# Check XDP mode
ip link show eth0 | grep xdp

# Expected: xdpgeneric (copy mode)
# Desired: xdpoffload or xdpdrv (zero-copy)
```

**Solutions**:
- Wrong driver: Use ixgbe/i40e/ice/mlx5
- Insufficient memory: Increase locked memory limit
- Kernel too old: Upgrade to 5.10+

## References

- [XDP Performance Best Practices](https://www.kernel.org/doc/html/latest/networking/af_xdp.html#performance)
- [Linux Network Stack Tuning](https://www.kernel.org/doc/Documentation/networking/scaling.txt)
- [WRAITH Performance Report](../../docs/PERFORMANCE_REPORT.md)
- [WRAITH Benchmarks](../../benches/)

---

**Last Updated**: 2025-12-06
**WRAITH Version**: 0.9.0 Beta
**Sprint**: 11.5 - XDP Documentation & CLI
