# WRAITH CLI Examples

## Table of Contents

1. [Basic Setup](#basic-setup)
2. [Simple File Transfer](#simple-file-transfer)
3. [Batch Transfers](#batch-transfers)
4. [High-Performance Transfer](#high-performance-transfer)
5. [Stealth Mode](#stealth-mode)
6. [NAT Traversal](#nat-traversal)
7. [Monitoring and Diagnostics](#monitoring-and-diagnostics)
8. [Advanced Scenarios](#advanced-scenarios)

---

## Basic Setup

### Initial Configuration

```bash
# 1. Install WRAITH
sudo cp wraith /usr/local/bin/
sudo chmod +x /usr/local/bin/wraith

# 2. Create configuration directory
mkdir -p ~/.wraith

# 3. Generate identity keypair
wraith generate-key --output ~/.wraith/identity.key --encrypt
# Enter passphrase: ********
# Confirm passphrase: ********
# Identity keypair generated: wraith:0123456789abcdef...

# 4. Create configuration file
cat > ~/.wraith/config.toml <<'EOF'
[node]
identity_file = "~/.wraith/identity.key"
bind = "0.0.0.0:40000"

[transport]
enable_xdp = true
enable_io_uring = true
worker_threads = 4

[obfuscation]
padding_mode = "SizeClasses"
timing_mode = "Normal"
protocol_mimicry = "TLS"

[discovery]
enable_dht = true
bootstrap_nodes = [
    "dht.wraith.network:40000",
    "seed.wraith.network:40000"
]
enable_nat_traversal = true
relay_servers = [
    "relay1.wraith.network:40000",
    "relay2.wraith.network:40000"
]

[transfer]
chunk_size = 262144
max_concurrent_transfers = 10
max_concurrent_chunks = 64
download_dir = "~/Downloads/wraith"
enable_resume = true

[logging]
level = "Info"
enable_metrics = true
EOF

# 5. Grant capabilities (for XDP support)
sudo setcap \
  cap_net_raw=ep,cap_net_admin=ep,cap_bpf=ep,cap_ipc_lock=ep \
  /usr/local/bin/wraith

# 6. Verify installation
wraith --version
# wraith 0.9.0

wraith info
# Node ID: wraith:0123456789abcdef...
```

---

## Simple File Transfer

### Scenario: Send a report to a colleague

**On Sender (Alice):**

```bash
# 1. Start daemon
wraith daemon &
# [INFO] WRAITH daemon ready
# [INFO] Node ID: wraith:alice123...

# 2. Get peer ID from colleague (Bob)
# Bob's Node ID: wraith:bob456...

# 3. Send file
wraith send quarterly-report.pdf --to wraith:bob456...
# [INFO] Connecting to peer wraith:bob456...
# [INFO] Session established
# [INFO] Sending file: quarterly-report.pdf (5.2 MB)
# [████████████████████████████████] 100% | 5.2 MB/5.2 MB | 9.3 Gbps | ETA: 0s
# [INFO] Transfer complete: 5.2 MB in 0.4s (13.0 MB/s)
# [INFO] File hash: blake3:a1b2c3d4e5f6...
```

**On Receiver (Bob):**

```bash
# 1. Start daemon
wraith daemon &
# [INFO] WRAITH daemon ready
# [INFO] Node ID: wraith:bob456...

# 2. Start receiving
wraith receive --output ~/Downloads/work
# [INFO] Listening for incoming transfers...
# [INFO] Incoming transfer from wraith:alice123...
# Accept transfer of quarterly-report.pdf (5.2 MB)? [y/N]: y
# [INFO] Receiving file: quarterly-report.pdf (5.2 MB)
# [████████████████████████████████] 100% | 5.2 MB/5.2 MB | 9.3 Gbps | ETA: 0s
# [INFO] Transfer complete: 5.2 MB in 0.4s (13.0 MB/s)
# [INFO] File saved: ~/Downloads/work/quarterly-report.pdf
# [INFO] File hash: blake3:a1b2c3d4e5f6... (verified)
```

---

## Batch Transfers

### Scenario: Send project files to team member

**Sender:**

```bash
# 1. Prepare files
ls ~/project/release/
# README.md  LICENSE  app-v1.0.0.tar.gz  checksums.txt

# 2. Send batch
wraith batch \
  --to wraith:teammate789... \
  --files ~/project/release/*

# [INFO] Batch transfer: 4 files, 125 MB total
# [INFO] Connecting to peer wraith:teammate789...
# [INFO] Session established
# [INFO] [1/4] README.md (15 KB)
# [████████████████████████████████] 100% | 15 KB/15 KB | 9.5 Gbps
# [INFO] [2/4] LICENSE (1.2 KB)
# [████████████████████████████████] 100% | 1.2 KB/1.2 KB | 9.5 Gbps
# [INFO] [3/4] app-v1.0.0.tar.gz (125 MB)
# [████████████████████████████████] 100% | 125 MB/125 MB | 9.4 Gbps
# [INFO] [4/4] checksums.txt (256 B)
# [████████████████████████████████] 100% | 256 B/256 B | 9.5 Gbps
# [INFO] Batch transfer complete: 125 MB in 13.3s (9.4 MB/s)
```

**Receiver:**

```bash
# Auto-accept from trusted peer
wraith receive \
  --output ~/Downloads/project-release \
  --auto-accept \
  --trusted-peers ~/.wraith/trusted-peers.txt

# [INFO] Listening for incoming transfers...
# [INFO] Incoming batch transfer from wraith:teammate789... (trusted, auto-accepting)
# [INFO] Receiving 4 files, 125 MB total
# [INFO] [1/4] README.md (15 KB)
# [████████████████████████████████] 100% | 15 KB/15 KB | 9.5 Gbps
# [INFO] [2/4] LICENSE (1.2 KB)
# [████████████████████████████████] 100% | 1.2 KB/1.2 KB | 9.5 Gbps
# [INFO] [3/4] app-v1.0.0.tar.gz (125 MB)
# [████████████████████████████████] 100% | 125 MB/125 MB | 9.4 Gbps
# [INFO] [4/4] checksums.txt (256 B)
# [████████████████████████████████] 100% | 256 B/256 B | 9.5 Gbps
# [INFO] Batch transfer complete: 125 MB in 13.3s (9.4 MB/s)
# [INFO] All files saved to ~/Downloads/project-release/
```

---

## High-Performance Transfer

### Scenario: Transfer large dataset (100 GB) at maximum speed

**Configuration for Maximum Throughput:**

```bash
# 1. Optimize configuration
cat > ~/.wraith/config-performance.toml <<'EOF'
[node]
identity_file = "~/.wraith/identity.key"
bind = "0.0.0.0:40000"

[transport]
enable_xdp = true           # Zero-copy kernel bypass
enable_io_uring = true      # Async file I/O
worker_threads = 8          # Use all cores

[obfuscation]
# Disable obfuscation for maximum speed
padding_mode = "None"
timing_mode = "None"
protocol_mimicry = "None"

[transfer]
chunk_size = 1048576        # 1 MB chunks (larger = faster)
max_concurrent_transfers = 20
max_concurrent_chunks = 128

[logging]
level = "Warn"              # Reduce logging overhead
enable_metrics = true
EOF

# 2. Increase system limits
ulimit -l unlimited         # Locked memory
ulimit -n 65536             # File descriptors

# 3. Pin to NUMA node 0 (if multi-socket)
NUMA_NODE=$(cat /sys/class/net/eth0/device/numa_node)
echo "NIC is on NUMA node: $NUMA_NODE"

# 4. Start daemon with optimizations
numactl --cpunodebind=$NUMA_NODE --membind=$NUMA_NODE \
  wraith --config ~/.wraith/config-performance.toml daemon

# 5. Send large file
wraith send dataset-100gb.tar.zst --to wraith:receiver...
# [INFO] Sending file: dataset-100gb.tar.zst (100 GB)
# [████████████████████████████████] 100% | 100 GB/100 GB | 38 Gbps | ETA: 0s
# [INFO] Transfer complete: 100 GB in 21.3s (4.7 GB/s, 37.6 Gbps)
```

**System Tuning:**

```bash
# Increase network buffer sizes
sudo sysctl -w net.core.rmem_max=134217728  # 128 MB
sudo sysctl -w net.core.wmem_max=134217728  # 128 MB

# Increase NIC ring sizes
sudo ethtool -G eth0 rx 4096 tx 4096

# Enable huge pages
echo 128 | sudo tee /proc/sys/vm/nr_hugepages

# Optimize interrupt coalescing (high throughput)
sudo ethtool -C eth0 rx-usecs 50 tx-usecs 50
```

---

## Stealth Mode

### Scenario: Maximum obfuscation for sensitive transfer

**Configuration for Maximum Stealth:**

```bash
# 1. Stealth configuration
cat > ~/.wraith/config-stealth.toml <<'EOF'
[node]
identity_file = "~/.wraith/identity.key"
bind = "0.0.0.0:443"        # Use HTTPS port

[transport]
enable_xdp = true
enable_io_uring = true

[obfuscation]
# Maximum obfuscation
padding_mode = "Statistical"     # Statistical padding (most realistic)
timing_mode = "Exponential"      # Exponential timing (mimics real traffic)
protocol_mimicry = "TLS"         # TLS protocol mimicry

[discovery]
enable_dht = true
bootstrap_nodes = ["dht.example.com:443"]
enable_nat_traversal = true
relay_servers = ["relay.example.com:443"]

[logging]
level = "Error"             # Minimal logging
enable_metrics = false      # Disable metrics collection
EOF

# 2. Start daemon
wraith --config ~/.wraith/config-stealth.toml daemon

# 3. Send file
wraith send sensitive-document.pdf --to wraith:recipient...
# [INFO] Session established (TLS mimicry active)
# [INFO] Sending file: sensitive-document.pdf (2.5 MB)
# [████████████████████████████████] 100% | 2.5 MB/2.5 MB | 5.2 Gbps
# [INFO] Transfer complete: 2.5 MB in 3.8s (658 KB/s)
# Note: Slower due to obfuscation overhead
```

**Traffic Analysis:**

```bash
# What an observer sees (tcpdump):
sudo tcpdump -i eth0 -nn 'port 443'
# 14:32:15.123456 IP 192.168.1.100.443 > 10.0.0.50.443: TLS 1.3 Application Data
# 14:32:15.234567 IP 10.0.0.50.443 > 192.168.1.100.443: TLS 1.3 Application Data
# ...
# Looks like HTTPS traffic!
```

---

## NAT Traversal

### Scenario: Connect peers behind different NATs

**Peer A (behind NAT 1):**

```bash
# 1. Start daemon with NAT traversal
wraith daemon --bind 0.0.0.0:40000
# [INFO] WRAITH daemon ready
# [INFO] Node ID: wraith:peerA123...
# [INFO] NAT Status: Full Cone NAT (STUN successful)
# [INFO] Public Address: 203.0.113.10:12345 (mapped)
# [INFO] Relay: Connected to relay1.wraith.network:40000

# 2. Check NAT status
wraith info | grep NAT
# NAT Status: Full Cone NAT
# Public Address: 203.0.113.10:12345

# 3. Send file to Peer B (also behind NAT)
wraith send report.pdf --to wraith:peerB456...
# [INFO] Connecting to peer wraith:peerB456...
# [INFO] Direct connection failed, attempting NAT traversal...
# [INFO] STUN: Public address: 203.0.113.10:12345
# [INFO] ICE: Gathering candidates...
# [INFO] ICE: Testing connectivity...
# [INFO] ICE: UDP hole punch successful!
# [INFO] Session established (direct peer-to-peer)
# [INFO] Sending file: report.pdf (5.2 MB)
# [████████████████████████████████] 100% | 5.2 MB/5.2 MB | 9.1 Gbps
```

**Peer B (behind NAT 2):**

```bash
# 1. Start daemon
wraith daemon --bind 0.0.0.0:40000
# [INFO] WRAITH daemon ready
# [INFO] Node ID: wraith:peerB456...
# [INFO] NAT Status: Symmetric NAT (relay required)
# [INFO] Public Address: 198.51.100.25:54321 (mapped)
# [INFO] Relay: Connected to relay1.wraith.network:40000

# 2. Receive file
wraith receive
# [INFO] Listening for incoming transfers...
# [INFO] Incoming connection from wraith:peerA123...
# [INFO] NAT traversal in progress...
# [INFO] ICE: UDP hole punch successful!
# [INFO] Session established (direct peer-to-peer)
# Accept transfer of report.pdf (5.2 MB)? [y/N]: y
# [████████████████████████████████] 100% | 5.2 MB/5.2 MB | 9.1 Gbps
```

**Fallback to Relay (if hole punching fails):**

```bash
# Peer A
wraith send report.pdf --to wraith:peerB456...
# [INFO] Connecting to peer wraith:peerB456...
# [INFO] Direct connection failed, attempting NAT traversal...
# [INFO] ICE: UDP hole punch failed (symmetric NAT)
# [WARN] Falling back to relay: relay1.wraith.network:40000
# [INFO] Session established (relayed)
# [INFO] Sending file: report.pdf (5.2 MB)
# [████████████████████████████████] 100% | 5.2 MB/5.2 MB | 2.8 Gbps
# Note: Slower due to relay hop
```

---

## Monitoring and Diagnostics

### Real-Time Status Monitoring

```bash
# Watch status (updates every 1 second)
watch -n1 'wraith status'

# Watch metrics
wraith metrics --watch --interval 1

# Watch specific transfer
watch -n1 'wraith status --transfer xfer_0123456789abcdef'
```

### Health Check Script

```bash
#!/bin/bash
# wraith-healthcheck.sh

set -e

echo "=== WRAITH Health Check ==="
echo "Timestamp: $(date)"
echo ""

# Check daemon is running
if ! pgrep -x wraith >/dev/null; then
    echo "❌ FAILED: WRAITH daemon not running"
    exit 1
fi
echo "✅ Daemon: Running"

# Check health
HEALTH=$(wraith health 2>&1)
if echo "$HEALTH" | grep -q "Overall: OK"; then
    echo "✅ Health: OK"
else
    echo "❌ FAILED: Health check failed"
    echo "$HEALTH"
    exit 1
fi

# Check active sessions
SESSIONS=$(wraith status 2>&1 | grep "Active Sessions" | awk '{print $3}')
echo "✅ Active Sessions: $SESSIONS"

# Check metrics
CPU=$(wraith metrics --json 2>&1 | jq -r '.resources.cpu_percent')
MEM=$(wraith metrics --json 2>&1 | jq -r '.resources.memory_bytes')
MEM_MB=$((MEM / 1024 / 1024))
echo "✅ CPU: ${CPU}%"
echo "✅ Memory: ${MEM_MB} MB"

# Check zero-copy
ZEROCOPY=$(wraith status --detailed 2>&1 | grep "Zero-Copy" | grep -q "Enabled" && echo "Yes" || echo "No")
echo "✅ Zero-Copy: $ZEROCOPY"

echo ""
echo "=== Health Check Complete ==="
```

### Performance Benchmarking

```bash
#!/bin/bash
# wraith-benchmark.sh

echo "=== WRAITH Performance Benchmark ==="

# Create test file (1 GB)
dd if=/dev/zero of=/tmp/test-1gb.bin bs=1M count=1024

# Get peer ID
read -p "Enter peer ID: " PEER_ID

# Benchmark 1: Single large file
echo ""
echo "Benchmark 1: Single 1 GB file transfer"
START=$(date +%s)
wraith send /tmp/test-1gb.bin --to "$PEER_ID"
END=$(date +%s)
DURATION=$((END - START))
THROUGHPUT=$((1024 / DURATION))
echo "Duration: ${DURATION}s"
echo "Throughput: ${THROUGHPUT} MB/s ($((THROUGHPUT * 8)) Mbps)"

# Benchmark 2: Multiple small files
echo ""
echo "Benchmark 2: 100 x 10 MB files"
mkdir -p /tmp/wraith-bench
for i in {1..100}; do
    dd if=/dev/zero of=/tmp/wraith-bench/file-$i.bin bs=1M count=10 2>/dev/null
done
START=$(date +%s)
wraith batch --to "$PEER_ID" --files /tmp/wraith-bench/*
END=$(date +%s)
DURATION=$((END - START))
THROUGHPUT=$((1000 / DURATION))
echo "Duration: ${DURATION}s"
echo "Throughput: ${THROUGHPUT} MB/s ($((THROUGHPUT * 8)) Mbps)"

# Cleanup
rm -rf /tmp/test-1gb.bin /tmp/wraith-bench

echo ""
echo "=== Benchmark Complete ==="
```

---

## Advanced Scenarios

### Multi-Peer Distribution

Send the same file to multiple peers simultaneously:

```bash
# Create peer list
cat > ~/.wraith/distribution-list.txt <<EOF
wraith:peer1...
wraith:peer2...
wraith:peer3...
wraith:peer4...
wraith:peer5...
EOF

# Send to all peers
while read -r peer; do
    wraith send large-dataset.tar.zst --to "$peer" &
done < ~/.wraith/distribution-list.txt

# Wait for all transfers
wait
echo "Distribution complete to all peers"
```

### Automated Backup

```bash
#!/bin/bash
# wraith-backup.sh - Daily backup to remote peer

BACKUP_PEER="wraith:backup-server..."
BACKUP_DIR="/home/user/important-data"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
ARCHIVE="/tmp/backup-${TIMESTAMP}.tar.zst"

echo "Creating backup archive..."
tar -I zstd -cf "$ARCHIVE" "$BACKUP_DIR"

echo "Sending backup to remote peer..."
wraith send "$ARCHIVE" --to "$BACKUP_PEER"

echo "Verifying transfer..."
# Hash is verified automatically by WRAITH

echo "Cleaning up..."
rm "$ARCHIVE"

echo "Backup complete: backup-${TIMESTAMP}.tar.zst"
```

### Continuous Monitoring Dashboard

```bash
#!/bin/bash
# wraith-dashboard.sh - Live monitoring dashboard

while true; do
    clear
    echo "════════════════════════════════════════════════════════════════"
    echo "                   WRAITH Live Dashboard"
    echo "════════════════════════════════════════════════════════════════"
    echo ""

    # Node info
    NODE_ID=$(wraith info 2>&1 | grep "Node ID" | awk '{print $3}')
    UPTIME=$(wraith status 2>&1 | grep "Uptime" | cut -d: -f2-)
    echo "Node ID: $NODE_ID"
    echo "Uptime: $UPTIME"
    echo ""

    # Sessions
    SESSIONS=$(wraith status 2>&1 | grep "Active Sessions" | awk '{print $3}')
    echo "Active Sessions: $SESSIONS"
    echo ""

    # Transfers
    echo "Active Transfers:"
    wraith status 2>&1 | grep -A 10 "Active Transfers:" | tail -n +2
    echo ""

    # Metrics
    METRICS=$(wraith metrics --json 2>&1)
    TX_BPS=$(echo "$METRICS" | jq -r '.transport.bytes_sent')
    RX_BPS=$(echo "$METRICS" | jq -r '.transport.bytes_received')
    CPU=$(echo "$METRICS" | jq -r '.resources.cpu_percent')
    MEM=$(echo "$METRICS" | jq -r '.resources.memory_bytes')

    echo "Network:"
    echo "  TX: $(numfmt --to=iec-i --suffix=B $TX_BPS)"
    echo "  RX: $(numfmt --to=iec-i --suffix=B $RX_BPS)"
    echo "  CPU: ${CPU}%"
    echo "  Memory: $(numfmt --to=iec-i --suffix=B $MEM)"
    echo ""

    echo "════════════════════════════════════════════════════════════════"
    echo "Press Ctrl+C to exit"

    sleep 2
done
```

---

## See Also

- [CLI Usage Guide](usage.md) - Complete CLI reference
- [Quick Reference](quick-reference.md) - Command cheat sheet
- [XDP Performance Guide](../xdp/performance.md) - Performance optimization
- [Troubleshooting Guide](../xdp/troubleshooting.md) - Common issues and solutions

---

**Last Updated**: 2025-12-06
**WRAITH Version**: 0.9.0 Beta
