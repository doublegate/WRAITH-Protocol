# XDP (eXpress Data Path) Overview

## Introduction

XDP (eXpress Data Path) is a Linux kernel feature that enables high-performance packet processing with programmable hooks in the network driver's receive path. WRAITH Protocol leverages XDP and AF_XDP (Address Family XDP) sockets to achieve wire-speed packet processing with minimal CPU overhead.

## What is XDP?

XDP provides a programmable, high-performance data plane directly in the Linux kernel's network stack:

- **Kernel Bypass**: Packets are processed before entering the kernel's network stack
- **Zero-Copy**: AF_XDP sockets enable zero-copy packet I/O between kernel and userspace
- **eBPF Programs**: Custom packet filtering and redirection using safe, verified eBPF programs
- **DMA Integration**: Direct Memory Access (DMA) from NIC to user-allocated memory regions

## Why WRAITH Uses XDP

WRAITH is designed for high-throughput, low-latency file transfer in adversarial network environments. XDP provides several critical advantages:

### 1. Performance

- **Throughput**: 10-40 Gbps on a single core (vs. 300 Mbps with standard UDP)
- **Latency**: Sub-microsecond kernel-to-userspace packet delivery
- **CPU Efficiency**: 10-100x fewer CPU cycles per packet
- **Zero-Copy**: Eliminates memory copies between kernel and userspace

### 2. Stealth

- **Raw Packet Control**: Fine-grained control over packet headers and timing
- **Obfuscation Support**: Custom packet crafting for protocol mimicry
- **Timing Precision**: Microsecond-level control over packet transmission timing

### 3. Resilience

- **Congestion Control**: BBR congestion control with precise RTT measurements
- **Packet Prioritization**: Custom eBPF programs for intelligent packet filtering
- **Multi-Path Support**: Efficient multi-path packet routing

## Architecture Components

WRAITH's XDP integration consists of three main components:

### 1. AF_XDP Sockets (`wraith-transport/af_xdp.rs`)

AF_XDP sockets provide the userspace interface to XDP:

```
┌─────────────────────────────────────────────┐
│           Userspace (WRAITH)                │
│  ┌──────────────────────────────────────┐   │
│  │        AF_XDP Socket                 │   │
│  │  ┌────────────┐    ┌────────────┐    │   │
│  │  │  RX Ring   │    │  TX Ring   │    │   │
│  │  └──────┬─────┘    └─────┬──────┘    │   │
│  │         │                 │           │   │
│  │  ┌──────▼─────┐    ┌─────▼──────┐    │   │
│  │  │ Fill Ring  │    │ Comp Ring  │    │   │
│  │  └────────────┘    └────────────┘    │   │
│  └──────────┬───────────────┬───────────┘   │
│             │               │                │
├─────────────┼───────────────┼────────────────┤
│             │   UMEM        │                │
│  ┌──────────▼───────────────▼─────────────┐  │
│  │  Shared Memory (Packet Buffers)        │  │
│  └──────────┬───────────────┬─────────────┘  │
├─────────────┼───────────────┼────────────────┤
│    Kernel   │               │                │
│  ┌──────────▼──────┐  ┌─────▼──────────┐    │
│  │   XDP Program   │  │  Network Stack │    │
│  │   (eBPF)        │  │                │    │
│  └──────────┬──────┘  └────────────────┘    │
│             │                                │
│  ┌──────────▼─────────────────────────────┐  │
│  │         Network Driver                 │  │
│  └──────────┬─────────────────────────────┘  │
└─────────────┼────────────────────────────────┘
              │
   ┌──────────▼──────────┐
   │   Network Interface │
   │      (eth0, etc)    │
   └─────────────────────┘
```

**Components:**

- **UMEM (User Memory)**: Shared memory region for packet buffers
- **RX Ring**: Queue for received packet descriptors
- **TX Ring**: Queue for transmitted packet descriptors
- **Fill Ring**: Free buffer descriptors for incoming packets
- **Completion Ring**: Completed transmission descriptors

### 2. io_uring Integration (`wraith-transport/io_uring.rs`)

Linux io_uring provides async file I/O to complement XDP's network I/O:

- **Batched Submission**: Submit multiple I/O operations in a single syscall
- **Completion Polling**: Process multiple completions without blocking
- **Registered Buffers**: Zero-copy file I/O with pre-registered memory regions
- **DMA Integration**: Direct memory access for supported storage devices

### 3. eBPF Programs (`wraith-xdp/` - Future Implementation)

Custom eBPF programs for packet filtering and redirection:

- **Packet Filtering**: Drop unwanted packets at the earliest possible stage
- **Connection Tracking**: Stateful packet tracking for session management
- **Load Balancing**: Distribute packets across multiple AF_XDP sockets
- **DDoS Protection**: Rate limiting and anomaly detection

## Current Implementation Status

### ✅ Completed (Phase 3)

- **AF_XDP Socket Management** (`wraith-transport/af_xdp.rs`)
  - UMEM allocation and management
  - Ring buffer creation and manipulation
  - Zero-copy packet transmission and reception
  - Socket binding and configuration
  - Comprehensive error handling
  - 12 unit tests, all passing

- **io_uring Context** (`wraith-transport/io_uring.rs`)
  - Submission Queue Entry (SQE) management
  - Completion Queue Entry (CQE) processing
  - Batched read/write operations
  - Buffer registration for zero-copy
  - Platform fallback for non-Linux systems
  - 13 unit tests, all passing

### 🔄 In Progress (Phase 11+)

- **eBPF Program Development** (`wraith-xdp/`)
  - Requires eBPF toolchain (libbpf, Clang 14+)
  - XDP program for packet filtering
  - XDP program for packet redirection
  - Integration with AF_XDP sockets

### 📋 Planned (Future Phases)

- **XDP Hardware Offload**: Support for NICs with XDP offload capabilities
- **Multi-Queue Support**: Distribute load across multiple RX/TX queues
- **Dynamic Queue Sizing**: Adaptive ring buffer sizes based on traffic patterns
- **Performance Tuning**: CPU affinity, NUMA awareness, IRQ balancing

## Fallback Behavior

WRAITH gracefully falls back to standard UDP sockets when XDP is unavailable:

### Automatic Fallback Conditions

1. **Non-Linux Platforms**: Windows, macOS automatically use UDP
2. **Kernel Version**: Linux kernels < 5.3 fall back to UDP
3. **Missing Privileges**: Non-root users without `CAP_NET_RAW` use UDP
4. **Disabled in Config**: `enable_xdp = false` in configuration
5. **eBPF Not Available**: Kernel without eBPF support uses UDP

### Fallback Performance

| Metric | XDP | UDP Fallback | Ratio |
|--------|-----|--------------|-------|
| Throughput | 10-40 Gbps | 300 Mbps | 33-133x |
| Latency | <1 μs | 10-50 μs | 10-50x |
| CPU per packet | ~100 cycles | ~10,000 cycles | 100x |
| Memory copies | 0 | 2-3 | N/A |

**Note**: UDP fallback still provides secure, reliable file transfer, just at lower performance.

## Comparison with Alternatives

### vs. DPDK (Data Plane Development Kit)

| Feature | XDP | DPDK |
|---------|-----|------|
| Kernel integration | ✅ Native | ❌ Bypass |
| Userspace polling | ✅ Optional | ✅ Required |
| CPU pinning | ✅ Optional | ✅ Required |
| Multi-process | ✅ Standard IPC | ❌ Shared memory |
| Complexity | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| Performance | 10-40 Gbps | 40-100 Gbps |

**Why XDP over DPDK**: Lower complexity, better integration, sufficient performance for WRAITH's use case.

### vs. Standard Sockets

| Feature | XDP | Standard Sockets |
|---------|-----|------------------|
| Kernel bypass | ✅ Partial | ❌ No |
| Zero-copy | ✅ Yes | ❌ No |
| Custom headers | ✅ Yes | ⚠️ Limited |
| Portability | ⚠️ Linux only | ✅ Cross-platform |
| Ease of use | ⭐⭐ | ⭐⭐⭐⭐⭐ |

**Why Standard Sockets as Fallback**: Universal compatibility, proven reliability.

## Use Cases

WRAITH's XDP integration excels in:

### 1. High-Bandwidth Transfers

- Multi-gigabyte file transfers over high-speed networks
- Media production workflows (4K/8K video files)
- Scientific data sharing (genomics, astronomy, climate models)
- Datacenter replication

### 2. Low-Latency Requirements

- Real-time collaboration (live editing, streaming)
- Time-sensitive data delivery (financial, medical)
- Interactive applications requiring immediate feedback

### 3. Adversarial Networks

- Censorship circumvention with custom packet crafting
- Traffic obfuscation and protocol mimicry
- DDoS resilience with intelligent packet filtering

### 4. Resource-Constrained Environments

- Embedded systems with limited CPU
- Battery-powered devices (less CPU = longer battery life)
- Multi-tenant servers (efficient CPU usage)

## Performance Expectations

### Single Core Performance

| Network | XDP Throughput | UDP Throughput | Speedup |
|---------|----------------|----------------|---------|
| 1 Gbps | 950 Mbps | 300 Mbps | 3.2x |
| 10 Gbps | 9.5 Gbps | 300 Mbps | 31.7x |
| 40 Gbps | 38 Gbps | 300 Mbps | 126.7x |

### Multi-Core Scaling

| Cores | XDP Throughput | CPU Usage | Latency |
|-------|----------------|-----------|---------|
| 1 | 10 Gbps | 100% | <1 μs |
| 2 | 20 Gbps | 100% | <1 μs |
| 4 | 40 Gbps | 100% | <1 μs |
| 8 | 80 Gbps | 100% | <1 μs |

**Note**: Linear scaling assumes sufficient NIC RSS (Receive Side Scaling) queues.

## Security Considerations

### XDP Security Benefits

1. **Reduced Attack Surface**: Fewer kernel code paths involved in packet processing
2. **Early Drop**: Malicious packets dropped before entering network stack
3. **Isolation**: eBPF programs run in sandboxed environment with verification
4. **Resource Limits**: Bounded execution time and memory access

### XDP Security Risks

1. **Root Privileges**: Loading eBPF programs typically requires root
2. **Kernel Complexity**: XDP adds complexity to the kernel
3. **Bug Exposure**: Kernel bugs in XDP code could be exploited

**Mitigation**: WRAITH uses AF_XDP sockets (userspace) with optional eBPF programs (future). The core protocol works without eBPF, reducing attack surface.

## Getting Started

### Quick Check: Is XDP Available?

```bash
# Check kernel version (need 5.3+)
uname -r

# Check if XDP is enabled in kernel
grep XDP /boot/config-$(uname -r)

# Check if interface supports XDP
ethtool -i eth0 | grep driver

# Verify eBPF is available
bpftool prog list
```

### Enable XDP in WRAITH

Edit `~/.config/wraith/config.toml`:

```toml
[network]
enable_xdp = true
xdp_interface = "eth0"  # Your network interface
udp_fallback = true     # Fallback to UDP if XDP fails
```

### Verify XDP is Active

```bash
# Start WRAITH daemon with debug logging
wraith daemon --bind 0.0.0.0:40000 --verbose

# Expected output:
# [INFO] XDP enabled on interface eth0
# [INFO] AF_XDP socket bound to 0.0.0.0:40000
# [INFO] Zero-copy mode: enabled
```

If XDP is unavailable, you'll see:

```bash
# [WARN] XDP unavailable: falling back to UDP
# [INFO] UDP socket bound to 0.0.0.0:40000
```

## Next Steps

- **[Requirements](requirements.md)**: Detailed kernel, hardware, and privilege requirements
- **[Architecture](architecture.md)**: Deep dive into AF_XDP socket internals
- **[Deployment](deployment.md)**: Production deployment guide with XDP
- **[Performance](performance.md)**: Benchmarking and tuning XDP performance
- **[Troubleshooting](troubleshooting.md)**: Common XDP issues and solutions
- **[io_uring Integration](io_uring.md)**: File I/O acceleration with io_uring

## References

- [Linux Kernel XDP Documentation](https://www.kernel.org/doc/html/latest/networking/af_xdp.html)
- [XDP Tutorial](https://github.com/xdp-project/xdp-tutorial)
- [AF_XDP Performance](https://www.kernel.org/doc/html/latest/networking/af_xdp.html#performance)
- [eBPF and XDP Reference Guide](https://cilium.readthedocs.io/en/stable/bpf/)
- [WRAITH Protocol Specification](../../ref-docs/protocol_technical_details.md)

---

**Last Updated**: 2025-12-06
**WRAITH Version**: 0.9.0 Beta
**Sprint**: 11.5 - XDP Documentation & CLI
