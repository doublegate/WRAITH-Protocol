# XDP Requirements

## Overview

This document details the hardware, software, and privilege requirements for using AF_XDP in WRAITH Protocol. Understanding these requirements helps determine whether XDP can be enabled in your environment.

## Summary Table

| Requirement | Minimum | Recommended | Notes |
|-------------|---------|-------------|-------|
| **Linux Kernel** | 5.3 | 6.2+ | Earlier kernels have limited XDP support |
| **NIC Driver** | Any (copy mode) | ixgbe, i40e, ice, mlx5 (zero-copy) | See [Driver Support](#driver-support) |
| **CPU** | x86_64 | x86_64 with AES-NI | ARM64 supported but less tested |
| **RAM** | 8 GB | 16+ GB | UMEM allocation requires locked pages |
| **Privileges** | CAP_NET_RAW + CAP_BPF | root | See [Privilege Requirements](#privilege-requirements) |
| **Tools** | None (copy mode) | libbpf, bpftool, Clang 14+ (eBPF) | For custom XDP programs |

## Linux Kernel Requirements

### Minimum Kernel Version

**5.3+** - Basic AF_XDP support

Earlier kernel versions (4.18-5.2) have partial XDP support but lack critical features:
- 4.18-4.19: Initial AF_XDP (unstable, not recommended)
- 5.0-5.2: Improved AF_XDP (missing zero-copy for some drivers)
- **5.3+**: Stable AF_XDP with broad driver support
- **5.10+**: LTS with mature XDP ecosystem
- **6.1+**: Current LTS with performance improvements
- **6.2+**: Recommended for WRAITH (latest features and fixes)

### Check Your Kernel Version

```bash
uname -r
# Expected output: 6.2.0 or higher
```

### Required Kernel Configuration

XDP requires specific kernel config options. Check your kernel config:

```bash
# Check kernel config (common locations)
grep -E "XDP|BPF" /boot/config-$(uname -r) | grep -v "^#"
```

**Required options**:
```
CONFIG_XDP_SOCKETS=y           # AF_XDP socket support
CONFIG_BPF=y                   # BPF subsystem
CONFIG_BPF_SYSCALL=y           # BPF syscall interface
CONFIG_BPF_JIT=y               # BPF Just-In-Time compiler (performance)
CONFIG_HAVE_EBPF_JIT=y         # Architecture supports eBPF JIT
```

**Optional but recommended**:
```
CONFIG_BPF_EVENTS=y            # BPF events for tracing
CONFIG_DEBUG_INFO_BTF=y        # BTF debug info (for bpftool)
CONFIG_XDP_SOCKETS_DIAG=y      # XDP socket diagnostics
CONFIG_NET_CLS_BPF=y           # BPF classifier (for tc integration)
CONFIG_NET_ACT_BPF=y           # BPF action (for tc integration)
```

### Kernel Modules

Some drivers require specific kernel modules:

```bash
# Load XDP socket module (usually auto-loaded)
sudo modprobe xsk

# Verify module is loaded
lsmod | grep xsk
```

## Hardware Requirements

### CPU Architecture

**Supported**:
- x86_64 (Intel, AMD) - Primary platform, fully tested
- aarch64 (ARM64) - Supported, less tested
- risc-v - Experimental support in recent kernels

**Not Supported**:
- x86 (32-bit) - XDP requires 64-bit addressing
- Other architectures - May work but untested

### CPU Features

**Required**:
- 64-bit CPU
- At least 2 cores (1 for network I/O, 1 for protocol processing)

**Recommended**:
- 4+ cores for multi-queue parallelism
- AES-NI for crypto acceleration (WRAITH uses XChaCha20-Poly1305)
- AVX2/AVX-512 for BLAKE3 hashing speedup

### Network Interface Card (NIC)

#### Driver Support

AF_XDP support varies by driver:

**Tier 1 (Zero-Copy + Full Features)**:
- `ixgbe` - Intel 82599, X520, X540, X550 (10 GbE)
- `i40e` - Intel X710, XL710, XXV710 (10/25/40 GbE)
- `ice` - Intel E810 (10/25/100 GbE)
- `mlx5` - Mellanox ConnectX-4/5/6 (10/25/40/100 GbE)

**Tier 2 (Zero-Copy, Limited Features)**:
- `igb` - Intel I350, I210 (1 GbE)
- `ixgbevf` - Intel 82599 VF (virtualized)
- `mlx4` - Mellanox ConnectX-3 (10/40 GbE)

**Tier 3 (Copy Mode Only)**:
- `e1000e` - Intel Gigabit Desktop/Mobile adapters
- `virtio_net` - QEMU/KVM virtual NIC (with vhost-net)
- Most other drivers (works but slower)

#### Check Your NIC Driver

```bash
# Find network interface
ip link show

# Check driver
ethtool -i eth0 | grep driver
# Expected output: driver: ixgbe (or i40e, ice, mlx5, etc.)
```

#### Verify XDP Support

```bash
# Check if driver supports XDP
ethtool -k eth0 | grep -i xdp

# Expected output:
# hw-tc-offload: off [fixed]
# xdp-offload: off [fixed]  (HW offload - rare)
# tx-gre-segmentation: on
```

If your driver doesn't appear in `ethtool -k` output, it may still support XDP in copy mode.

#### Multi-Queue Support

XDP performs best with multi-queue NICs (RSS - Receive Side Scaling):

```bash
# Check number of RX/TX queues
ethtool -l eth0

# Expected output:
# Channel parameters for eth0:
# Pre-set maximums:
# RX:             8
# TX:             8
# Combined:       8
```

**Recommendation**: Use at least as many queues as CPU cores for optimal parallelism.

#### Configure Queue Count

```bash
# Set combined queues (RX + TX)
sudo ethtool -L eth0 combined 4

# Verify
ethtool -l eth0
```

### Memory Requirements

#### RAM

**Minimum**: 8 GB total system RAM

**Recommended**: 16+ GB for high-throughput workloads

**Calculation**:
```
UMEM size = 4 MB (default, configurable)
Per-queue UMEM = 4 MB × num_queues
Total XDP memory = UMEM × num_sockets

Example (4 queues, 2 sockets):
4 MB × 4 × 2 = 32 MB

This is small, but memory must be locked (non-swappable).
```

#### Locked Memory Limit

XDP requires locking pages in RAM (preventing swap):

```bash
# Check current locked memory limit
ulimit -l
# Expected: 65536 (64 KB) - too small for XDP!

# Set unlimited locked memory (temporary)
ulimit -l unlimited

# Verify
ulimit -l
# Expected: unlimited
```

**Make permanent** (add to `/etc/security/limits.conf`):
```
# /etc/security/limits.conf
*    soft    memlock    unlimited
*    hard    memlock    unlimited
```

**Or** grant capability to specific user:
```bash
# Grant CAP_IPC_LOCK to user
sudo setcap cap_ipc_lock=ep /usr/local/bin/wraith
```

#### Huge Pages (Optional)

For maximum performance, use huge pages for UMEM:

```bash
# Check current huge pages
cat /proc/meminfo | grep Huge

# Allocate 128 x 2 MB huge pages (256 MB total)
echo 128 | sudo tee /proc/sys/vm/nr_hugepages

# Verify
grep HugePages_Total /proc/meminfo
```

## Privilege Requirements

XDP requires elevated privileges due to:
- Raw network access (sending/receiving arbitrary packets)
- Kernel memory pinning (locking pages)
- eBPF program loading (kernel code injection)

### Option 1: Run as Root (Easiest)

```bash
# Run WRAITH daemon as root
sudo wraith daemon --bind 0.0.0.0:40000

# Or use sudo for all commands
sudo wraith send file.dat --to <peer-id>
```

**Pros**: Simple, works everywhere

**Cons**: Security risk (full root access)

### Option 2: Capabilities (Recommended)

Grant specific capabilities instead of full root:

```bash
# Grant required capabilities
sudo setcap \
    cap_net_raw=ep,cap_net_admin=ep,cap_bpf=ep,cap_ipc_lock=ep \
    /usr/local/bin/wraith

# Verify capabilities
getcap /usr/local/bin/wraith
# Expected output:
# /usr/local/bin/wraith cap_bpf,cap_ipc_lock,cap_net_admin,cap_net_raw=ep
```

**Required capabilities**:
- `CAP_NET_RAW` - Create AF_XDP sockets, send/receive raw packets
- `CAP_NET_ADMIN` - Configure network interfaces (bind XDP programs)
- `CAP_BPF` - Load eBPF programs (for future XDP filtering)
- `CAP_IPC_LOCK` - Lock memory pages (for UMEM)

**Pros**: Fine-grained permissions, better security

**Cons**: Requires recent kernel (5.8+) for `CAP_BPF`

### Option 3: User Namespace (Advanced)

Run WRAITH in a user namespace with mapped capabilities:

```bash
# Create user namespace with network capabilities
unshare -r -n wraith daemon --bind 0.0.0.0:40000
```

**Pros**: No persistent privilege escalation

**Cons**: Complex setup, limited portability

### Capability Fallback

If capabilities are unavailable, WRAITH automatically falls back to:
1. Standard UDP sockets (no XDP)
2. Non-elevated permissions
3. Reduced performance but still functional

Check WRAITH logs:
```
[WARN] XDP unavailable: insufficient privileges (need CAP_NET_RAW)
[INFO] Falling back to UDP transport
```

## Software Requirements

### Build Dependencies

**Required** (for building WRAITH):
- Rust 1.88+ (2024 Edition)
- Cargo (Rust package manager)
- GCC or Clang (C compiler for linking)

**Optional** (for eBPF programs):
- Clang 14+ (LLVM-based C compiler with eBPF backend)
- libbpf (BPF CO-RE library)
- bpftool (BPF introspection and debugging)
- linux-headers (kernel headers for eBPF compilation)

### Runtime Dependencies

**Required**:
- Linux kernel 5.3+ with XDP support
- libc (glibc or musl)

**Optional**:
- bpftool (for XDP diagnostics)
- ethtool (for NIC configuration)
- iproute2 (for `ip` command, network setup)

### Install Build Tools (Debian/Ubuntu)

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build dependencies
sudo apt update
sudo apt install -y build-essential pkg-config

# Install eBPF toolchain (optional)
sudo apt install -y clang llvm libbpf-dev linux-headers-$(uname -r)

# Install runtime tools
sudo apt install -y bpftool ethtool iproute2
```

### Install Build Tools (Fedora/RHEL/CentOS)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build dependencies
sudo dnf install -y gcc gcc-c++ pkg-config

# Install eBPF toolchain (optional)
sudo dnf install -y clang llvm libbpf-devel kernel-devel

# Install runtime tools
sudo dnf install -y bpftool ethtool iproute
```

### Install Build Tools (Arch Linux)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install all dependencies
sudo pacman -S base-devel clang llvm libbpf bpf ethtool iproute2
```

## Verification Checklist

Use this checklist to verify your system meets XDP requirements:

### 1. Kernel Version

```bash
uname -r
# ✅ 6.2.0 or higher
# ⚠️ 5.3.0 to 6.1.x (works but not optimal)
# ❌ < 5.3.0 (insufficient XDP support)
```

### 2. Kernel Config

```bash
zgrep -E "CONFIG_XDP_SOCKETS|CONFIG_BPF_SYSCALL" /proc/config.gz
# ✅ CONFIG_XDP_SOCKETS=y
# ✅ CONFIG_BPF_SYSCALL=y
```

### 3. NIC Driver

```bash
ethtool -i eth0 | grep driver
# ✅ driver: ixgbe / i40e / ice / mlx5 (zero-copy capable)
# ⚠️ driver: e1000e / virtio_net (copy mode only)
```

### 4. Multi-Queue Support

```bash
ethtool -l eth0 | grep Combined
# ✅ Combined: 4 or higher
# ⚠️ Combined: 1 (single queue, limited parallelism)
```

### 5. Locked Memory Limit

```bash
ulimit -l
# ✅ unlimited
# ❌ 65536 or lower (insufficient)
```

### 6. Capabilities

```bash
getcap $(which wraith) 2>/dev/null || echo "No capabilities set"
# ✅ cap_net_raw,cap_net_admin,cap_bpf,cap_ipc_lock=ep
# ⚠️ No capabilities set (run as root or grant capabilities)
```

### 7. bpftool (Optional)

```bash
bpftool version
# ✅ bpftool v6.2.0 or higher
# ⚠️ command not found (optional, but useful for debugging)
```

## Platform-Specific Notes

### Debian/Ubuntu

- Kernel 5.15+ available in Ubuntu 22.04 LTS (recommended)
- Install `linux-generic-hwe-22.04` for latest kernel on 22.04
- XDP support enabled by default in recent kernels

### Fedora

- Latest kernels (6.x) available out-of-box
- eBPF toolchain readily available in repositories
- Excellent XDP support

### RHEL/CentOS

- RHEL 8.x: Kernel 4.18 (insufficient for XDP)
- RHEL 9.x: Kernel 5.14+ (XDP supported)
- Consider using Fedora for development, RHEL 9+ for production

### Arch Linux

- Rolling release with latest kernels (6.x)
- Bleeding-edge XDP support
- Excellent for development and testing

### Cloud Platforms

#### AWS EC2

- **Instance types**: C5n, C6i, M5n (enhanced networking)
- **NIC**: ENA (Elastic Network Adapter) - supports XDP copy mode
- **Zero-copy**: Not supported (virtualized NIC)
- **Performance**: ~5 Gbps per vCPU with XDP copy mode

#### Google Cloud (GCP)

- **Instance types**: C2, C3 (compute-optimized)
- **NIC**: virtio-net - supports XDP copy mode
- **Zero-copy**: Not supported (virtualized NIC)
- **Performance**: ~5-10 Gbps with XDP copy mode

#### Azure

- **Instance types**: Fv2, Dasv5 (accelerated networking)
- **NIC**: Mellanox ConnectX-3/4/5 (SR-IOV) - **supports zero-copy XDP**
- **Zero-copy**: **Supported with accelerated networking enabled**
- **Performance**: ~10-20 Gbps with zero-copy XDP

**Recommendation**: Azure with accelerated networking for best cloud XDP performance.

### Virtualization

#### KVM/QEMU

- **NIC**: virtio-net with vhost-net - supports XDP copy mode
- **Zero-copy**: Not supported in guest (requires SR-IOV passthrough)
- **Performance**: ~2-5 Gbps with XDP copy mode
- **SR-IOV passthrough**: Enables zero-copy XDP (physical NIC exposed to guest)

#### Docker/Podman

- **Host networking** (`--network=host`): Full XDP support (use host NIC)
- **Bridge networking**: XDP not supported (packet forwarding breaks zero-copy)
- **Macvlan**: XDP copy mode supported (limited performance)

**Recommendation**: Use host networking for XDP in containers.

#### LXC/LXD

- Similar to Docker: host networking recommended for XDP
- Unprivileged containers: XDP not supported (insufficient privileges)
- Privileged containers: XDP supported (equivalent to host)

## Troubleshooting Requirements

### Issue: "XDP not supported on this kernel"

**Diagnosis**:
```bash
uname -r
# Check if kernel is 5.3+
```

**Solution**: Upgrade kernel to 5.3 or later.

### Issue: "AF_XDP socket creation failed"

**Diagnosis**:
```bash
# Check if XDP sockets are enabled
zgrep CONFIG_XDP_SOCKETS /proc/config.gz
```

**Solution**: Rebuild kernel with `CONFIG_XDP_SOCKETS=y` or use a distribution kernel with XDP enabled.

### Issue: "Cannot lock memory (ENOMEM)"

**Diagnosis**:
```bash
ulimit -l
# Should be unlimited or at least 1048576 (1 GB)
```

**Solution**: Increase locked memory limit (see [Locked Memory Limit](#locked-memory-limit)).

### Issue: "Permission denied (EPERM)"

**Diagnosis**:
```bash
getcap $(which wraith)
# Check if capabilities are set
```

**Solution**: Grant capabilities or run as root (see [Privilege Requirements](#privilege-requirements)).

### Issue: "Driver does not support XDP"

**Diagnosis**:
```bash
ethtool -i eth0 | grep driver
# Check driver name
```

**Solution**: XDP copy mode should still work. Zero-copy requires specific drivers (see [Driver Support](#driver-support)).

## Next Steps

- **[Overview](overview.md)**: Learn about XDP and why WRAITH uses it
- **[Architecture](architecture.md)**: Understand AF_XDP internals
- **[Deployment](deployment.md)**: Deploy WRAITH with XDP in production
- **[Performance](performance.md)**: Benchmark and optimize XDP performance
- **[Troubleshooting](troubleshooting.md)**: Solve common XDP issues

---

**Last Updated**: 2025-12-06
**WRAITH Version**: 0.9.0 Beta
**Sprint**: 11.5 - XDP Documentation & CLI
