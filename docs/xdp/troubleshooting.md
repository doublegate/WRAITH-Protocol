# XDP Troubleshooting Guide

## Overview

This guide provides solutions to common issues when using AF_XDP in WRAITH Protocol. Issues are organized by symptom with diagnostic steps and solutions.

## Quick Diagnostics

Run these commands to gather system information:

```bash
# Create diagnostic report
cat > /tmp/wraith-xdp-diag.sh <<'EOF'
#!/bin/bash
echo "=== WRAITH XDP Diagnostics ==="
echo "Date: $(date)"
echo ""

echo "=== Kernel Version ==="
uname -a
echo ""

echo "=== Kernel Config (XDP) ==="
zgrep -E "CONFIG_XDP|CONFIG_BPF" /proc/config.gz 2>/dev/null || echo "Config not available"
echo ""

echo "=== Network Interface ==="
ip link show
echo ""

echo "=== NIC Driver ==="
for iface in $(ip -o link show | awk -F': ' '{print $2}'); do
  echo "$iface: $(ethtool -i $iface 2>/dev/null | grep driver || echo 'N/A')"
done
echo ""

echo "=== NIC Queues ==="
for iface in $(ip -o link show | awk -F': ' '{print $2}'); do
  echo "$iface:"
  ethtool -l $iface 2>/dev/null || echo "  N/A"
done
echo ""

echo "=== Locked Memory Limit ==="
ulimit -l
echo ""

echo "=== Capabilities (wraith binary) ==="
which wraith >/dev/null 2>&1 && getcap $(which wraith) || echo "wraith not in PATH"
echo ""

echo "=== Running Processes ==="
ps aux | grep -E "wraith|xdp" | grep -v grep
echo ""

echo "=== Recent Errors ==="
journalctl -u wraith --no-pager -n 50 | grep -i error || echo "No errors found"
echo ""

echo "=== XDP Programs Loaded ==="
bpftool prog show 2>/dev/null || echo "bpftool not available"
echo ""

echo "=== Diagnostics complete ==="
EOF

chmod +x /tmp/wraith-xdp-diag.sh
/tmp/wraith-xdp-diag.sh
```

## Common Issues

### 1. XDP Initialization Fails

#### Symptom

```
[ERROR] AF_XDP socket creation failed: Operation not permitted (os error 1)
[WARN] XDP unavailable: falling back to UDP
```

#### Diagnosis

```bash
# Check if running as root or with capabilities
id
getcap $(which wraith)

# Check kernel support
zgrep CONFIG_XDP_SOCKETS /proc/config.gz
```

#### Solutions

**A. Grant Capabilities** (Recommended):
```bash
sudo setcap \
  cap_net_raw=ep,cap_net_admin=ep,cap_bpf=ep,cap_ipc_lock=ep \
  /usr/local/bin/wraith
```

**B. Run as Root** (Not recommended for production):
```bash
sudo wraith daemon --bind 0.0.0.0:40000
```

**C. Rebuild Kernel with XDP Support**:
```bash
# Enable in kernel config
CONFIG_XDP_SOCKETS=y
CONFIG_BPF_SYSCALL=y

# Rebuild and install
make && sudo make modules_install install
```

---

### 2. Zero-Copy Mode Not Enabled

#### Symptom

```
[INFO] AF_XDP socket bound to 0.0.0.0:40000
[WARN] Zero-copy mode: disabled (using copy mode)
```

#### Diagnosis

```bash
# Check NIC driver
ethtool -i eth0 | grep driver

# Check XDP mode
ip link show eth0 | grep xdp
```

#### Solutions

**A. Use Compatible NIC Driver**:

Zero-copy requires specific drivers:
- `ixgbe` (Intel X520, X540, X550)
- `i40e` (Intel X710, XL710)
- `ice` (Intel E810)
- `mlx5` (Mellanox ConnectX-4/5/6)

**B. Verify Sufficient Resources**:
```bash
# Check locked memory limit
ulimit -l  # Should be 'unlimited'

# Check available memory
free -h
```

**C. Accept Copy Mode**:

Copy mode still provides significant performance improvement over standard UDP:
- Copy mode: 1-5 Gbps
- UDP: ~300 Mbps

Zero-copy is not required for most use cases.

---

### 3. Performance Lower Than Expected

#### Symptom

```
Throughput: 2 Gbps (expected: 9.5 Gbps)
```

#### Diagnosis

```bash
# Check for packet drops
ethtool -S eth0 | grep -i drop

# Check CPU usage
mpstat -P ALL 1 5

# Check interrupt distribution
cat /proc/interrupts | grep eth0

# Check queue count
ethtool -l eth0
```

#### Solutions

**A. Enable More Queues**:
```bash
# Set to number of CPU cores
sudo ethtool -L eth0 combined 4
```

**B. Optimize Interrupt Coalescing**:
```bash
# For high throughput
sudo ethtool -C eth0 rx-usecs 50 tx-usecs 50

# For low latency
sudo ethtool -C eth0 rx-usecs 1 tx-usecs 1

# Balanced (WRAITH default)
sudo ethtool -C eth0 rx-usecs 10 tx-usecs 10
```

**C. Pin CPUs**:
```bash
# Isolate CPUs for WRAITH
# Add to /etc/default/grub:
GRUB_CMDLINE_LINUX="isolcpus=0-3 nohz_full=0-3"

# Update grub and reboot
sudo update-grub && sudo reboot

# Run WRAITH on isolated CPUs
wraith daemon --cpus 0-3
```

**D. Increase Ring Sizes**:
```bash
# Increase NIC ring sizes
sudo ethtool -G eth0 rx 4096 tx 4096

# Increase WRAITH ring sizes
wraith daemon --rx-ring-size 4096 --tx-ring-size 4096
```

**E. Enable Huge Pages**:
```bash
# Allocate huge pages
echo 128 | sudo tee /proc/sys/vm/nr_hugepages

# Run WRAITH with huge pages
wraith daemon --umem-huge-pages
```

---

### 4. High Latency / Jitter

#### Symptom

```
RTT: 500 μs (expected: <10 μs)
Jitter: 200 μs (expected: <50 μs)
```

#### Diagnosis

```bash
# Check context switches
pidstat -w -p $(pgrep wraith) 1 5

# Check page faults
pidstat -r -p $(pgrep wraith) 1 5

# Check IRQ distribution
cat /proc/interrupts | grep eth0
```

#### Solutions

**A. Reduce Context Switches**:
```bash
# Increase process priority
sudo nice -n -20 wraith daemon

# Or use systemd
[Service]
Nice=-20
```

**B. Lock Memory**:
```bash
# Increase locked memory limit
ulimit -l unlimited

# Enable memory locking in WRAITH
wraith daemon --lock-memory
```

**C. Reduce Interrupt Coalescing**:
```bash
# Lower interrupt delay
sudo ethtool -C eth0 rx-usecs 1 tx-usecs 1
```

**D. Pin to NUMA Node**:
```bash
# Find NIC's NUMA node
cat /sys/class/net/eth0/device/numa_node

# Pin WRAITH to same node (e.g., node 0)
numactl --cpunodebind=0 --membind=0 wraith daemon
```

---

### 5. Packet Drops

#### Symptom

```bash
ethtool -S eth0 | grep -i drop
# rx_dropped: 1234567
# tx_dropped: 56789
```

#### Diagnosis

```bash
# Check ring sizes
ethtool -g eth0

# Check buffer sizes
sysctl net.core.rmem_max net.core.wmem_max

# Check interface errors
ip -s link show eth0
```

#### Solutions

**A. Increase Ring Sizes**:
```bash
# Increase both NIC and WRAITH rings
sudo ethtool -G eth0 rx 4096 tx 4096
wraith daemon --rx-ring-size 4096 --tx-ring-size 4096
```

**B. Increase System Buffers**:
```bash
# Increase network buffer sizes
sudo sysctl -w net.core.rmem_max=134217728  # 128 MB
sudo sysctl -w net.core.wmem_max=134217728  # 128 MB

# Make persistent
sudo tee -a /etc/sysctl.conf <<EOF
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
EOF
```

**C. Enable Batching**:
```bash
# Increase batch size (reduces overhead)
wraith daemon --batch-size 128
```

**D. Reduce Traffic Load**:
```bash
# Limit throughput temporarily
wraith send file.dat --to <peer-id> --rate 5G  # Limit to 5 Gbps
```

---

### 6. "Cannot bind to address" Error

#### Symptom

```
[ERROR] Failed to bind socket: Address already in use (os error 98)
```

#### Diagnosis

```bash
# Check if port is in use
sudo ss -ulnp | grep 40000

# Check for zombie processes
ps aux | grep wraith | grep -v grep
```

#### Solutions

**A. Kill Existing Process**:
```bash
# Find and kill
sudo pkill wraith

# Or kill specific PID
sudo kill $(pgrep wraith)
```

**B. Wait for Socket Timeout**:
```bash
# Wait 60 seconds for socket to close
sleep 60
wraith daemon --bind 0.0.0.0:40000
```

**C. Use Different Port**:
```bash
# Bind to different port
wraith daemon --bind 0.0.0.0:40001
```

**D. Enable SO_REUSEADDR**:

(Already enabled in WRAITH, but verify)

---

### 7. Memory Allocation Failures

#### Symptom

```
[ERROR] UMEM creation failed: Cannot allocate memory (os error 12)
```

#### Diagnosis

```bash
# Check locked memory limit
ulimit -l

# Check available memory
free -h

# Check UMEM size
grep -i umem /var/log/wraith.log
```

#### Solutions

**A. Increase Locked Memory Limit**:
```bash
# Set unlimited
ulimit -l unlimited

# Make persistent
sudo tee -a /etc/security/limits.conf <<EOF
*    soft    memlock    unlimited
*    hard    memlock    unlimited
EOF
```

**B. Reduce UMEM Size**:
```bash
# Use smaller UMEM (default: 4 MB)
wraith daemon --umem-size 2097152  # 2 MB
```

**C. Free System Memory**:
```bash
# Drop caches
sudo sync; echo 3 | sudo tee /proc/sys/vm/drop_caches
```

**D. Close Other Applications**:
```bash
# Free up memory by closing applications
# Or move WRAITH to a dedicated server
```

---

### 8. XDP Program Load Failures (Future)

#### Symptom

```
[ERROR] Failed to load XDP program: Invalid argument (os error 22)
```

**Note**: This applies to future eBPF XDP programs. WRAITH currently uses AF_XDP sockets only (no eBPF programs).

#### Diagnosis

```bash
# Check kernel version (need 5.3+)
uname -r

# Check BPF capabilities
bpftool feature probe | grep -i xdp

# Verify program with bpftool
bpftool prog load wraith_filter.o /sys/fs/bpf/wraith
```

#### Solutions

**A. Upgrade Kernel**:
```bash
# Upgrade to 5.10+ (LTS)
sudo apt install linux-generic-hwe-22.04
sudo reboot
```

**B. Fix eBPF Program**:
```bash
# Compile with debug info
clang -O2 -g -target bpf -c wraith_filter.c -o wraith_filter.o

# Verify with bpftool
bpftool prog dump xlated pinned /sys/fs/bpf/wraith
```

**C. Check Verifier Logs**:
```bash
# Get detailed verifier error
bpftool prog load wraith_filter.o /sys/fs/bpf/wraith 2>&1 | less
```

---

### 9. Virtualization Issues

#### Symptom

```
[WARN] XDP unavailable in virtualized environment
[INFO] Falling back to UDP
```

#### Diagnosis

```bash
# Check if virtualized
systemd-detect-virt

# Check NIC type
ethtool -i eth0 | grep driver
```

#### Solutions

**A. Use SR-IOV Passthrough** (KVM/QEMU):
```xml
<!-- libvirt XML -->
<interface type='hostdev'>
  <source>
    <address domain='0x0000' bus='0x03' slot='0x00' function='0x0'/>
  </source>
</interface>
```

**B. Enable Host Networking** (Docker):
```bash
docker run --network=host wraith:latest
```

**C. Accept Copy Mode**:

Most virtual NICs support XDP copy mode:
- VirtIO (QEMU/KVM): ✅ Copy mode supported
- ENA (AWS EC2): ✅ Copy mode supported
- gVNIC (GCP): ✅ Copy mode supported
- Hyper-V NetVSC (Azure): ✅ Copy mode supported (zero-copy with SR-IOV!)

---

### 10. SELinux / AppArmor Denials

#### Symptom

```
[ERROR] Permission denied when accessing /dev/xdp
audit: type=1400 audit(1234567890.123:456): avc: denied { read } for comm="wraith" ...
```

#### Diagnosis

```bash
# Check SELinux status
getenforce

# Check for denials
ausearch -m avc -c wraith | tail -20

# Check AppArmor status
aa-status | grep wraith
```

#### Solutions

**A. Create SELinux Policy**:
```bash
# Generate policy from denials
ausearch -m avc -c wraith | audit2allow -M wraith-xdp

# Install policy
sudo semodule -i wraith-xdp.pp
```

**B. Create AppArmor Profile**:
```bash
# Create profile
sudo tee /etc/apparmor.d/usr.local.bin.wraith <<EOF
#include <tunables/global>

/usr/local/bin/wraith {
  #include <abstractions/base>
  #include <abstractions/nameservice>

  capability net_raw,
  capability net_admin,
  capability bpf,
  capability ipc_lock,

  network packet raw,
  /sys/class/net/** r,
  /proc/sys/net/** r,
}
EOF

# Load profile
sudo apparmor_parser -r /etc/apparmor.d/usr.local.bin.wraith
```

**C. Disable (Not Recommended)**:
```bash
# Disable SELinux temporarily
sudo setenforce 0

# Or disable AppArmor temporarily
sudo aa-complain /usr/local/bin/wraith
```

---

## Debugging Tools

### 1. bpftool

Inspect XDP programs and maps (future):

```bash
# List loaded BPF programs
sudo bpftool prog show

# Dump XDP program
sudo bpftool prog dump xlated id <prog-id>

# List BPF maps
sudo bpftool map show

# Dump map contents
sudo bpftool map dump id <map-id>
```

### 2. ethtool

Configure and inspect NIC:

```bash
# Show driver info
ethtool -i eth0

# Show ring sizes
ethtool -g eth0

# Show queue configuration
ethtool -l eth0

# Show statistics
ethtool -S eth0

# Show offload settings
ethtool -k eth0
```

### 3. perf

Profile XDP performance:

```bash
# Record XDP events
sudo perf record -e xdp:xdp_devmap_xmit -a -g -- sleep 10

# Analyze results
sudo perf report
```

### 4. tcpdump

Capture packets (bypasses XDP, sees all traffic):

```bash
# Capture on interface
sudo tcpdump -i eth0 -w /tmp/capture.pcap

# Analyze with Wireshark
wireshark /tmp/capture.pcap
```

### 5. Strace

Trace syscalls:

```bash
# Trace wraith syscalls
sudo strace -f -e trace=network,desc wraith daemon

# Or attach to running process
sudo strace -f -p $(pgrep wraith)
```

## Getting Help

If you're still experiencing issues:

1. **Gather Diagnostics**:
   ```bash
   /tmp/wraith-xdp-diag.sh > /tmp/wraith-diag.txt 2>&1
   ```

2. **Check Logs**:
   ```bash
   journalctl -u wraith --no-pager -n 100 > /tmp/wraith-logs.txt
   ```

3. **Report Issue**:
   - Create GitHub issue: https://github.com/doublegate/WRAITH-Protocol/issues
   - Include diagnostic output and logs
   - Describe expected vs. actual behavior
   - List steps to reproduce

4. **Community Support**:
   - Join Discord: [link]
   - Matrix room: [link]
   - Mailing list: [link]

## References

- [XDP Overview](overview.md)
- [XDP Requirements](requirements.md)
- [XDP Deployment](deployment.md)
- [XDP Performance](performance.md)
- [Linux XDP Documentation](https://www.kernel.org/doc/html/latest/networking/af_xdp.html)
- [XDP Troubleshooting Guide](https://github.com/xdp-project/xdp-tutorial/tree/master/packet03-redirecting)

---

**Last Updated**: 2025-12-06
**WRAITH Version**: 0.9.0 Beta
**Sprint**: 11.5 - XDP Documentation & CLI
