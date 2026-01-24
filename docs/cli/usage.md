# WRAITH CLI Usage Guide

## Overview

The WRAITH command-line interface (CLI) provides a user-friendly interface to the WRAITH Protocol's secure file transfer and peer-to-peer networking capabilities.

## Table of Contents

1. [Installation](#installation)
2. [Basic Usage](#basic-usage)
3. [Configuration](#configuration)
4. [Daemon Mode](#daemon-mode)
5. [File Transfer](#file-transfer)
6. [Peer Management](#peer-management)
7. [Network Status](#network-status)
8. [Health and Metrics](#health-and-metrics)
9. [Advanced Features](#advanced-features)
10. [Troubleshooting](#troubleshooting)

---

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/doublegate/WRAITH-Protocol.git
cd WRAITH-Protocol

# Build release binary
cargo build --release

# Install to system (optional)
sudo cp target/release/wraith /usr/local/bin/
```

### Binary Release

```bash
# Download latest release
curl -LO https://github.com/doublegate/WRAITH-Protocol/releases/latest/download/wraith-linux-x86_64

# Make executable
chmod +x wraith-linux-x86_64

# Move to PATH
sudo mv wraith-linux-x86_64 /usr/local/bin/wraith
```

### Verify Installation

```bash
wraith --version
# Output: wraith 0.9.0
```

---

## Basic Usage

### Command Structure

```bash
wraith [OPTIONS] <COMMAND> [ARGS]
```

### Global Options

- `--verbose, -v`: Enable verbose output (debug level)
- `--debug, -d`: Enable debug output (trace level, implies --verbose)
- `--help, -h`: Print help information
- `--version, -V`: Print version information

### Available Commands

| Command | Description |
|---------|-------------|
| `daemon` | Start WRAITH daemon |
| `send` | Send file(s) to peer |
| `receive` | Receive files from peers |
| `batch` | Send multiple files to peer |
| `peers` | List connected peers |
| `status` | Show connection status |
| `health` | Check node health |
| `metrics` | Display node metrics |
| `info` | Show node information |
| `ping` | Ping a peer |
| `config` | Manage configuration |
| `generate-key` | Generate identity keypair |

---

## Configuration

### Generate Identity

Before using WRAITH, generate an identity keypair:

```bash
# Generate unencrypted keypair
wraith generate-key --output ~/.wraith/identity.key

# Generate encrypted keypair (recommended)
wraith generate-key --output ~/.wraith/identity.key --encrypt
# Enter passphrase: ********
# Confirm passphrase: ********
```

### Configuration File

Create `~/.wraith/config.toml`:

```toml
[node]
# Node identity
identity_file = "~/.wraith/identity.key"

# Network binding
bind = "0.0.0.0:40000"

[transport]
# Enable kernel bypass (requires root or CAP_NET_RAW)
enable_xdp = true
enable_io_uring = true

# Worker threads (default: CPU count)
worker_threads = 4

# Connection timeouts
connection_timeout_secs = 30
idle_timeout_secs = 300

[obfuscation]
# Padding mode: None, PowerOfTwo, SizeClasses, ConstantRate, Statistical
padding_mode = "SizeClasses"

# Timing mode: None, Fixed, Uniform, Normal, Exponential
timing_mode = "Normal"

# Protocol mimicry: None, TLS, WebSocket, DoH
protocol_mimicry = "TLS"

[discovery]
# Enable DHT peer discovery
enable_dht = true

# Bootstrap nodes (example)
bootstrap_nodes = [
    "dht.wraith.network:40000",
    "seed.wraith.network:40000"
]

# Enable NAT traversal
enable_nat_traversal = true

# Relay servers (for NAT hole punching)
relay_servers = [
    "relay1.wraith.network:40000",
    "relay2.wraith.network:40000"
]

[transfer]
# Chunk size for file transfers (256 KiB)
chunk_size = 262144

# Maximum concurrent transfers
max_concurrent_transfers = 10

# Maximum concurrent chunks per transfer
max_concurrent_chunks = 64

# Download directory
download_dir = "~/Downloads/wraith"

# Enable resume support
enable_resume = true

[logging]
# Log level: Trace, Debug, Info, Warn, Error
level = "Info"

# Enable metrics collection
enable_metrics = true
```

### View Configuration

```bash
# Show current configuration
wraith config show

# Show configuration in JSON format
wraith config show --json
```

### Update Configuration

```bash
# Set configuration value
wraith config set node.bind "0.0.0.0:50000"

# Set obfuscation settings
wraith config set obfuscation.padding_mode "ConstantRate"
wraith config set obfuscation.timing_mode "Exponential"
```

---

## Daemon Mode

### Start Daemon

```bash
# Start with default configuration
wraith daemon

# Start with custom bind address
wraith daemon --bind 0.0.0.0:40000

# Start with specific number of workers
wraith daemon --bind 0.0.0.0:40000 --workers 8

# Start in debug mode
wraith --debug daemon --bind 0.0.0.0:40000
```

### Daemon Output

```
[INFO] WRAITH Protocol v0.9.0
[INFO] Loading identity from /home/user/.wraith/identity.key
[INFO] Node ID: wraith:01234567890abcdef...
[INFO] Initializing transport layer...
[INFO] AF_XDP socket bound to 0.0.0.0:40000
[INFO] Zero-copy mode: enabled
[INFO] Worker threads: 4
[INFO] Initializing discovery layer...
[INFO] DHT enabled, bootstrapping from 2 nodes
[INFO] NAT traversal enabled
[INFO] WRAITH daemon ready
```

### Systemd Service

Create `/etc/systemd/system/wraith.service`:

```ini
[Unit]
Description=WRAITH Protocol Daemon
After=network.target

[Service]
Type=simple
User=wraith
Group=wraith
ExecStart=/usr/local/bin/wraith daemon --bind 0.0.0.0:40000
Restart=on-failure
RestartSec=10

# Security hardening
CapabilityBoundingSet=CAP_NET_RAW CAP_NET_ADMIN CAP_BPF CAP_IPC_LOCK
AmbientCapabilities=CAP_NET_RAW CAP_NET_ADMIN CAP_BPF CAP_IPC_LOCK
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/wraith

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable wraith
sudo systemctl start wraith
sudo systemctl status wraith
```

---

## File Transfer

### Send File

```bash
# Send file to peer by ID
wraith send report.pdf --to wraith:0123456789abcdef...

# Send with progress display
wraith send large-file.tar.gz --to wraith:0123456789abcdef...

# Send to multiple peers (multicast)
wraith send document.pdf \
  --to wraith:0123456789abcdef... \
  --to wraith:fedcba9876543210...
```

### Send Output

```
[INFO] Connecting to peer wraith:0123456789abcdef...
[INFO] Session established
[INFO] Sending file: report.pdf (10.5 MB)
[████████████████████████████████] 100% | 10.5 MB/10.5 MB | 9.2 Gbps | ETA: 0s
[INFO] Transfer complete: 10.5 MB in 0.9s (11.7 MB/s)
[INFO] File hash: blake3:a1b2c3d4e5f6...
```

### Batch Send

```bash
# Send multiple files
wraith batch \
  --to wraith:0123456789abcdef... \
  --files file1.txt file2.pdf file3.tar.gz

# Send directory (recursive)
wraith batch \
  --to wraith:0123456789abcdef... \
  --files ~/Documents/project/
```

### Batch Output

```
[INFO] Batch transfer: 3 files, 125 MB total
[INFO] Connecting to peer wraith:0123456789abcdef...
[INFO] Session established
[INFO] [1/3] file1.txt (1.2 MB)
[████████████████████████████████] 100% | 1.2 MB/1.2 MB | 9.5 Gbps | ETA: 0s
[INFO] [2/3] file2.pdf (10.8 MB)
[████████████████████████████████] 100% | 10.8 MB/10.8 MB | 9.4 Gbps | ETA: 0s
[INFO] [3/3] file3.tar.gz (113 MB)
[████████████████████████████████] 100% | 113 MB/113 MB | 9.6 Gbps | ETA: 0s
[INFO] Batch transfer complete: 125 MB in 13.2s (9.5 MB/s)
```

### Receive Files

```bash
# Receive files (daemon must be running)
wraith receive

# Receive with custom output directory
wraith receive --output ~/Downloads/wraith-incoming

# Receive with auto-accept from trusted peers
wraith receive --auto-accept --trusted-peers ~/.wraith/trusted.txt
```

### Receive Output

```
[INFO] Listening for incoming transfers...
[INFO] Incoming transfer from wraith:0123456789abcdef...
Accept transfer of report.pdf (10.5 MB)? [y/N]: y
[INFO] Receiving file: report.pdf (10.5 MB)
[████████████████████████████████] 100% | 10.5 MB/10.5 MB | 9.3 Gbps | ETA: 0s
[INFO] Transfer complete: 10.5 MB in 0.9s (11.7 MB/s)
[INFO] File saved: ~/Downloads/wraith/report.pdf
[INFO] File hash: blake3:a1b2c3d4e5f6... (verified)
```

---

## Peer Management

### List Peers

```bash
# List all connected peers
wraith peers

# List peers with DHT query
wraith peers --dht-query

# List peers with detailed information
wraith peers --verbose
```

### Peer List Output

```
Connected Peers (3):

1. wraith:0123456789abcdef...
   Address: 192.168.1.100:40000
   Connected: 5m 23s ago
   RTT: 2.5 ms
   Sent: 125 MB (1,234 packets)
   Received: 98 MB (987 packets)
   Loss: 0.01%

2. wraith:fedcba9876543210...
   Address: 10.0.0.50:40001
   Connected: 12m 8s ago
   RTT: 15.3 ms
   Sent: 45 MB (456 packets)
   Received: 67 MB (678 packets)
   Loss: 0.02%

3. wraith:abcdef0123456789...
   Address: 172.16.5.25:40000
   Connected: 2h 45m ago
   RTT: 1.2 ms
   Sent: 2.3 GB (23,456 packets)
   Received: 1.8 GB (18,234 packets)
   Loss: 0.00%
```

### DHT Query Output

```bash
wraith peers --dht-query
```

```
DHT Peers (15):

Local Peers (3):
- wraith:0123456789abcdef... (192.168.1.100:40000) - Connected
- wraith:fedcba9876543210... (10.0.0.50:40001) - Connected
- wraith:abcdef0123456789... (172.16.5.25:40000) - Connected

DHT Discovered (12):
- wraith:111111111111... (dht.wraith.network:40000) - Bootstrap
- wraith:222222222222... (203.0.113.10:40000) - Reachable
- wraith:333333333333... (198.51.100.25:40001) - NAT (relay available)
- wraith:444444444444... (192.0.2.50:40000) - Reachable
...
```

### Ping Peer

```bash
# Ping peer by ID
wraith ping wraith:0123456789abcdef...

# Ping with count
wraith ping wraith:0123456789abcdef... --count 10

# Ping with interval
wraith ping wraith:0123456789abcdef... --interval 500ms
```

### Ping Output

```
PING wraith:0123456789abcdef... (192.168.1.100:40000)
64 bytes from wraith:0123456789abcdef...: seq=1 time=2.5 ms
64 bytes from wraith:0123456789abcdef...: seq=2 time=2.3 ms
64 bytes from wraith:0123456789abcdef...: seq=3 time=2.6 ms
64 bytes from wraith:0123456789abcdef...: seq=4 time=2.4 ms

--- wraith:0123456789abcdef... ping statistics ---
4 packets transmitted, 4 received, 0% packet loss
rtt min/avg/max/mdev = 2.3/2.4/2.6/0.1 ms
```

---

## Network Status

### Connection Status

```bash
# Show overall connection status
wraith status

# Show specific transfer status
wraith status --transfer <transfer-id>

# Show detailed status with all sessions
wraith status --detailed
```

### Status Output

```
WRAITH Node Status

Node ID: wraith:0123456789abcdef...
Uptime: 3h 45m 12s
Network: Online

Transport:
  Mode: AF_XDP + io_uring (zero-copy enabled)
  Bind: 0.0.0.0:40000
  Workers: 4 threads

Connections:
  Active Sessions: 3
  Total Bytes Sent: 2.5 GB
  Total Bytes Received: 2.1 GB
  Average RTT: 5.2 ms
  Packet Loss: 0.01%

Discovery:
  DHT Status: Connected (15 peers)
  NAT Status: Direct (no NAT)
  Relay Status: Not needed

Active Transfers:
  1. report.pdf → wraith:fedcba9876543210... (45% complete, 9.2 Gbps)
  2. backup.tar.gz ← wraith:abcdef0123456789... (78% complete, 8.7 Gbps)

Health: OK
```

### Detailed Status

```bash
wraith status --detailed
```

```
WRAITH Node Status (Detailed)

Node Information:
  Node ID: wraith:0123456789abcdef01234567890abcdef01234567890abcdef0123456789
  Uptime: 3h 45m 12s
  Network: Online
  Version: 0.9.0

Transport Layer:
  Mode: AF_XDP + io_uring
  Zero-Copy: Enabled
  Bind Address: 0.0.0.0:40000
  Worker Threads: 4
  Ring Sizes: RX=2048, TX=2048
  UMEM Size: 4 MB

Network Statistics:
  Total Sessions: 12 (3 active, 9 closed)
  Total Bytes Sent: 2,543,891,456 (2.5 GB)
  Total Bytes Received: 2,187,654,321 (2.1 GB)
  Total Packets Sent: 25,439
  Total Packets Received: 21,877
  Packet Loss Rate: 0.01%
  Average RTT: 5.2 ms

Active Sessions:
  1. wraith:0123456789abcdef...
     Address: 192.168.1.100:40000
     Duration: 5m 23s
     Sent: 125 MB (1,234 packets)
     Received: 98 MB (987 packets)
     RTT: 2.5 ms, Loss: 0.01%

  2. wraith:fedcba9876543210...
     Address: 10.0.0.50:40001
     Duration: 12m 8s
     Sent: 45 MB (456 packets)
     Received: 67 MB (678 packets)
     RTT: 15.3 ms, Loss: 0.02%

  3. wraith:abcdef0123456789...
     Address: 172.16.5.25:40000
     Duration: 2h 45m
     Sent: 2.3 GB (23,456 packets)
     Received: 1.8 GB (18,234 packets)
     RTT: 1.2 ms, Loss: 0.00%

Discovery:
  DHT Status: Connected
  DHT Peers: 15 total (3 local, 12 discovered)
  Bootstrap Nodes: 2 connected
  NAT Status: Direct (no NAT detected)
  Public Address: 203.0.113.50:40000
  Relay Status: Not needed

Active Transfers:
  1. Transfer ID: xfer_0123456789abcdef
     File: report.pdf (10.5 MB)
     Direction: Outgoing
     Peer: wraith:fedcba9876543210...
     Progress: 45% (4.7 MB / 10.5 MB)
     Speed: 9.2 Gbps
     ETA: 1.2s
     Chunks: 18/41 complete

  2. Transfer ID: xfer_fedcba9876543210
     File: backup.tar.gz (250 MB)
     Direction: Incoming
     Peer: wraith:abcdef0123456789...
     Progress: 78% (195 MB / 250 MB)
     Speed: 8.7 Gbps
     ETA: 5.8s
     Chunks: 763/977 complete

Health Check:
  Overall: OK
  Transport: OK (zero-copy active)
  Discovery: OK (DHT connected)
  Sessions: OK (3 active)
  Transfers: OK (2 active)
  Memory: OK (156 MB / 4 GB used)
  CPU: OK (45% average)
```

### Transfer-Specific Status

```bash
wraith status --transfer xfer_0123456789abcdef
```

```
Transfer Status: xfer_0123456789abcdef

File: report.pdf
Size: 10.5 MB
Direction: Outgoing
Peer: wraith:fedcba9876543210...
Peer Address: 10.0.0.50:40001

Progress: 45% (4.7 MB / 10.5 MB)
Speed: 9.2 Gbps (current), 9.5 Gbps (average)
ETA: 1.2 seconds

Chunks:
  Total: 41 chunks (256 KiB each)
  Completed: 18 chunks
  In Progress: 4 chunks
  Pending: 19 chunks

Timeline:
  Started: 2025-12-06 14:32:15 UTC
  Duration: 0.5 seconds
  Estimated Completion: 2025-12-06 14:32:16 UTC

Integrity:
  Hash Algorithm: BLAKE3 (tree hashing)
  Completed Chunks Verified: 18/18 (100%)
  Tree Hash Progress: 45%
```

---

## Health and Metrics

### Health Check

```bash
# Check node health
wraith health
```

### Health Output

```
WRAITH Node Health Check

Overall: OK

Transport Layer:
  Status: OK
  Mode: AF_XDP + io_uring (zero-copy enabled)
  Socket: Bound to 0.0.0.0:40000
  Workers: 4/4 threads active

Discovery Layer:
  Status: OK
  DHT: Connected (15 peers)
  NAT: Direct (no NAT)
  Relay: Not needed

Session Manager:
  Status: OK
  Active Sessions: 3
  Session Errors: 0 (last hour)
  Average RTT: 5.2 ms

Transfer Manager:
  Status: OK
  Active Transfers: 2
  Transfer Errors: 0 (last hour)
  Average Speed: 9.0 Gbps

Resource Usage:
  Memory: 156 MB / 4 GB (3.9%)
  CPU: 45% average (last minute)
  Network: 9.2 Gbps TX, 8.7 Gbps RX

Warnings: None
Errors: None

Last Updated: 2025-12-06 14:32:45 UTC
```

### Display Metrics

```bash
# Show metrics (human-readable)
wraith metrics

# Show metrics in JSON format
wraith metrics --json

# Watch metrics (update every 2 seconds)
wraith metrics --watch --interval 2
```

### Metrics Output

```
WRAITH Node Metrics

Transport Layer:
  Packets Sent: 25,439 (1,234 pps)
  Packets Received: 21,877 (987 pps)
  Bytes Sent: 2.5 GB (125 MB/s)
  Bytes Received: 2.1 GB (98 MB/s)
  Packet Loss: 0.01%
  Zero-Copy TX: 98.5%
  Zero-Copy RX: 99.2%

Sessions:
  Total Sessions: 12
  Active Sessions: 3
  Sessions Opened (last hour): 5
  Sessions Closed (last hour): 2
  Average Session Duration: 45m 12s
  Session Errors: 0

Transfers:
  Total Transfers: 48
  Active Transfers: 2
  Completed Transfers: 46
  Failed Transfers: 0
  Total Bytes Transferred: 125 GB
  Average Transfer Speed: 9.3 Gbps

Discovery:
  DHT Peers: 15
  DHT Queries (last hour): 23
  DHT Lookups (last hour): 8
  NAT Traversal Attempts: 0
  Relay Connections: 0

Congestion Control (BBR):
  Bandwidth: 9.5 Gbps
  RTT: 5.2 ms (min: 1.2 ms)
  Inflight: 128 KB
  Pacing Rate: 10.0 Gbps

Crypto:
  Handshakes: 12 total (5 last hour)
  Ratchets: 156 (13 last hour)
  Encryptions: 25,439 packets
  Decryptions: 21,877 packets
  Crypto Errors: 0

Resource Usage:
  Memory: 156 MB
  CPU: 45%
  File Descriptors: 45 / 1024
  Threads: 12

Uptime: 3h 45m 12s
Last Updated: 2025-12-06 14:32:45 UTC
```

### Metrics JSON Output

```bash
wraith metrics --json
```

```json
{
  "timestamp": "2025-12-06T14:32:45Z",
  "uptime_seconds": 13512,
  "transport": {
    "packets_sent": 25439,
    "packets_received": 21877,
    "bytes_sent": 2543891456,
    "bytes_received": 2187654321,
    "packet_loss_rate": 0.0001,
    "zero_copy_tx_rate": 0.985,
    "zero_copy_rx_rate": 0.992,
    "pps_sent": 1234,
    "pps_received": 987
  },
  "sessions": {
    "total_sessions": 12,
    "active_sessions": 3,
    "sessions_opened_last_hour": 5,
    "sessions_closed_last_hour": 2,
    "average_session_duration_seconds": 2712,
    "session_errors": 0
  },
  "transfers": {
    "total_transfers": 48,
    "active_transfers": 2,
    "completed_transfers": 46,
    "failed_transfers": 0,
    "total_bytes_transferred": 134217728000,
    "average_transfer_speed_bps": 9300000000
  },
  "discovery": {
    "dht_peers": 15,
    "dht_queries_last_hour": 23,
    "dht_lookups_last_hour": 8,
    "nat_traversal_attempts": 0,
    "relay_connections": 0
  },
  "bbr": {
    "bandwidth_bps": 9500000000,
    "rtt_us": 5200,
    "min_rtt_us": 1200,
    "inflight_bytes": 131072,
    "pacing_rate_bps": 10000000000
  },
  "crypto": {
    "handshakes_total": 12,
    "handshakes_last_hour": 5,
    "ratchets_total": 156,
    "ratchets_last_hour": 13,
    "encryptions": 25439,
    "decryptions": 21877,
    "crypto_errors": 0
  },
  "resources": {
    "memory_bytes": 163577856,
    "cpu_percent": 45,
    "file_descriptors": 45,
    "file_descriptors_max": 1024,
    "threads": 12
  }
}
```

### Node Information

```bash
# Show node information
wraith info
```

### Info Output

```
WRAITH Node Information

Node Identity:
  Node ID: wraith:0123456789abcdef01234567890abcdef01234567890abcdef0123456789
  Ed25519 Public Key: 0123456789abcdef...
  X25519 Public Key: fedcba9876543210...
  Identity File: /home/user/.wraith/identity.key

Network:
  Bind Address: 0.0.0.0:40000
  Public Address: 203.0.113.50:40000 (detected via STUN)
  NAT Status: Direct (no NAT)
  Network Type: IPv4 + IPv6

Transport:
  Mode: AF_XDP + io_uring
  Zero-Copy: Enabled
  Socket Type: UDP + AF_XDP
  Worker Threads: 4
  Ring Sizes: RX=2048, TX=2048
  UMEM Size: 4 MB
  Huge Pages: Enabled

Capabilities:
  XDP Supported: Yes (zero-copy mode)
  io_uring Supported: Yes
  Huge Pages Available: Yes (128 x 2 MB)
  NUMA Nodes: 1
  CPU Cores: 8 (4 allocated)

Configuration:
  Config File: /home/user/.wraith/config.toml
  Download Directory: /home/user/Downloads/wraith
  Log Level: Info
  Metrics: Enabled

Protocol Version: 0.9.0
Build: v0.9.0-beta-g0123456 (2025-12-06)
Rust Version: 1.88.0
Platform: x86_64-unknown-linux-gnu
```

---

## Advanced Features

### Debug Mode

```bash
# Enable trace-level logging
wraith --debug daemon --bind 0.0.0.0:40000
```

Debug output includes:
- Detailed frame parsing (header fields, encryption metadata)
- Session state transitions
- BBR congestion control decisions
- DHT routing table updates
- NAT traversal attempts
- Crypto operations (handshakes, ratchets)

### Verbose Mode

```bash
# Enable debug-level logging
wraith --verbose send report.pdf --to wraith:0123456789abcdef...
```

Verbose output includes:
- Session establishment steps
- File chunking progress
- Transfer chunk acknowledgments
- Network statistics

### Custom Configuration

```bash
# Use custom config file
wraith --config /etc/wraith/custom.toml daemon

# Override specific settings
wraith daemon \
  --bind 0.0.0.0:50000 \
  --workers 8 \
  --config /etc/wraith/custom.toml
```

### Obfuscation Settings

```bash
# Configure padding mode
wraith config set obfuscation.padding_mode "ConstantRate"

# Configure timing mode
wraith config set obfuscation.timing_mode "Exponential"

# Configure protocol mimicry
wraith config set obfuscation.protocol_mimicry "TLS"
```

### Discovery Settings

```bash
# Enable DHT
wraith config set discovery.enable_dht true

# Add bootstrap nodes
wraith config set discovery.bootstrap_nodes '["dht.wraith.network:40000"]'

# Enable NAT traversal
wraith config set discovery.enable_nat_traversal true

# Add relay servers
wraith config set discovery.relay_servers '["relay1.wraith.network:40000"]'
```

---

## Troubleshooting

### Check Logs

```bash
# View daemon logs (systemd)
sudo journalctl -u wraith -f

# View daemon logs (manual)
tail -f ~/.wraith/wraith.log
```

### Common Issues

#### 1. Permission Denied (AF_XDP)

**Symptom:**
```
[ERROR] AF_XDP socket creation failed: Operation not permitted (os error 1)
[WARN] XDP unavailable: falling back to UDP
```

**Solution:**
```bash
# Grant capabilities (recommended)
sudo setcap cap_net_raw=ep,cap_net_admin=ep,cap_bpf=ep,cap_ipc_lock=ep /usr/local/bin/wraith

# Or run as root (not recommended)
sudo wraith daemon --bind 0.0.0.0:40000
```

#### 2. Connection Timeout

**Symptom:**
```
[ERROR] Failed to connect to peer: Connection timeout
```

**Solutions:**
- Check peer is online: `wraith ping wraith:0123456789abcdef...`
- Verify firewall allows UDP port 40000
- Check NAT configuration if behind NAT
- Verify peer's public address: `wraith peers --dht-query`

#### 3. Transfer Failures

**Symptom:**
```
[ERROR] Transfer failed: Integrity verification failed
```

**Solutions:**
- Retry transfer: WRAITH supports automatic resume
- Check network stability: `wraith status --detailed`
- Verify disk space: `df -h ~/Downloads/wraith`
- Check file permissions on sender

#### 4. DHT Discovery Issues

**Symptom:**
```
[WARN] DHT bootstrap failed: No response from bootstrap nodes
```

**Solutions:**
- Verify bootstrap nodes are reachable: `nc -zvu dht.wraith.network 40000`
- Check firewall allows UDP traffic
- Wait for discovery timeout (30 seconds)
- Add more bootstrap nodes in config

#### 5. Zero-Copy Not Enabled

**Symptom:**
```
[WARN] Zero-copy mode: disabled (using copy mode)
```

**Solutions:**
- Check NIC driver: `ethtool -i eth0` (need ixgbe, i40e, ice, mlx5)
- Verify locked memory limit: `ulimit -l` (should be unlimited)
- Check kernel version: `uname -r` (need 5.3+)
- See [XDP Troubleshooting Guide](../xdp/troubleshooting.md)

### Diagnostic Commands

```bash
# Run health check
wraith health

# Check node information
wraith info

# View detailed status
wraith status --detailed

# Check metrics
wraith metrics

# Test peer connectivity
wraith ping wraith:0123456789abcdef...
```

### Get Help

```bash
# General help
wraith --help

# Command-specific help
wraith send --help
wraith daemon --help
wraith config --help
```

### Report Issues

If you encounter a bug or issue:

1. Gather diagnostic information:
   ```bash
   wraith info > wraith-info.txt
   wraith status --detailed > wraith-status.txt
   wraith metrics --json > wraith-metrics.json
   sudo journalctl -u wraith -n 100 > wraith-logs.txt
   ```

2. Create GitHub issue: https://github.com/doublegate/WRAITH-Protocol/issues
3. Include diagnostic files and steps to reproduce

---

## See Also

- [XDP Overview](../xdp/overview.md) - XDP kernel bypass technology
- [XDP Deployment](../xdp/deployment.md) - Production deployment guide
- [XDP Troubleshooting](../xdp/troubleshooting.md) - XDP-specific issues
- [Configuration Reference](../CONFIG_REFERENCE.md) - Complete configuration options
- [User Guide](../USER_GUIDE.md) - Detailed user guide
- [Architecture](../architecture/README.md) - System architecture documentation

---

**Last Updated**: 2025-12-06
**WRAITH Version**: 0.9.0 Beta
**Sprint**: 11.5 - XDP Documentation & CLI
