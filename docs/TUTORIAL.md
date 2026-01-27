# WRAITH Protocol Tutorial

**Version:** 1.0.0
**Last Updated:** 2025-12-05
**Status:** Complete Tutorial Guide

---

## Table of Contents

1. [Getting Started](#1-getting-started)
2. [Basic Usage](#2-basic-usage)
3. [Configuration](#3-configuration)
4. [Advanced Topics](#4-advanced-topics)
5. [Security Best Practices](#5-security-best-practices)
6. [Examples](#6-examples)

---

## 1. Getting Started

### 1.1 Prerequisites

Before installing WRAITH Protocol, ensure your system meets these requirements:

**Operating System:**
- Linux 6.2+ (recommended for full AF_XDP and io_uring support)
- macOS 12+ (UDP mode only, no kernel bypass)
- Windows 10+ (experimental UDP support)

**Software Requirements:**
- Rust 1.88+ (for building from source)
- Build tools: gcc/clang, pkg-config, make
- libssl-dev (Ubuntu/Debian) or openssl-devel (RHEL/Fedora)

**Hardware Requirements:**
- x86_64 or aarch64 architecture
- 2+ CPU cores (4+ recommended for high-performance transfers)
- 4 GB RAM minimum (8 GB recommended for multiple concurrent transfers)
- Network interface with UDP support

**Optional (for kernel bypass):**
- Linux kernel 6.2+ with AF_XDP support
- NIC with AF_XDP driver support (Intel XL710, Mellanox ConnectX-5+)
- Root or CAP_NET_RAW capability

### 1.2 Installation from Pre-Built Binaries

The easiest way to install WRAITH is using pre-built binaries from the releases page.

**Linux (Debian/Ubuntu):**
```bash
# Download the latest release
wget https://github.com/doublegate/WRAITH-Protocol/releases/download/v0.9.0/wraith_0.9.0_amd64.deb

# Install
sudo dpkg -i wraith_0.9.0_amd64.deb

# Verify installation
wraith --version
# Output: wraith 0.9.0 (Rust 1.88, 2024 Edition)
```

**Linux (RHEL/Fedora/CentOS):**
```bash
# Download RPM package
wget https://github.com/doublegate/WRAITH-Protocol/releases/download/v0.9.0/wraith-0.9.0-1.x86_64.rpm

# Install
sudo rpm -i wraith-0.9.0-1.x86_64.rpm

# Verify
wraith --version
```

**Linux (Generic):**
```bash
# Download tarball
wget https://github.com/doublegate/WRAITH-Protocol/releases/download/v0.9.0/wraith-0.9.0-x86_64-unknown-linux-gnu.tar.gz

# Extract
tar xzf wraith-0.9.0-x86_64-unknown-linux-gnu.tar.gz

# Move to system path
sudo install -m 755 wraith /usr/local/bin/

# Verify
wraith --version
```

**macOS:**
```bash
# Download for your architecture
# Intel Macs:
wget https://github.com/doublegate/WRAITH-Protocol/releases/download/v0.9.0/wraith-0.9.0-x86_64-apple-darwin.tar.gz

# Apple Silicon:
wget https://github.com/doublegate/WRAITH-Protocol/releases/download/v0.9.0/wraith-0.9.0-aarch64-apple-darwin.tar.gz

# Extract and install
tar xzf wraith-*.tar.gz
sudo install -m 755 wraith /usr/local/bin/

# Verify
wraith --version
```

**Windows:**
```powershell
# Download from releases page using a web browser
# Or use PowerShell:
Invoke-WebRequest -Uri "https://github.com/doublegate/WRAITH-Protocol/releases/download/v0.9.0/wraith-0.9.0-x86_64-pc-windows-msvc.zip" -OutFile "wraith.zip"

# Extract
Expand-Archive -Path wraith.zip -DestinationPath C:\Program Files\WRAITH\

# Add to PATH (PowerShell as Administrator)
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Program Files\WRAITH\", [EnvironmentVariableTarget]::Machine)

# Verify (new terminal)
wraith --version
```

### 1.3 Building from Source

For the latest features or platform-specific optimizations, build from source:

**1. Install Rust:**
```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Ensure Rust 1.88+ is installed
rustup update stable
rustc --version
# Should show: rustc 1.88.0 or higher
```

**2. Install System Dependencies:**

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev git
```

**Fedora/RHEL:**
```bash
sudo dnf install -y gcc pkg-config openssl-devel git
```

**macOS:**
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew if not installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install openssl pkg-config
```

**3. Clone and Build:**
```bash
# Clone the repository
git clone https://github.com/doublegate/WRAITH-Protocol.git
cd WRAITH-Protocol

# Build release binary
cargo build --release

# Binary will be at: target/release/wraith
./target/release/wraith --version

# Optional: Install to system
sudo install -m 755 target/release/wraith /usr/local/bin/
```

**4. Run Tests (Optional):**
```bash
# Run the test suite
cargo test --workspace

# Should see: 2,120+ tests passing
```

### 1.4 First Run and Configuration

After installation, set up your WRAITH identity and configuration:

**1. Generate Your Identity:**
```bash
# Create ~/.config/wraith/ directory
mkdir -p ~/.config/wraith

# Generate a new keypair
wraith keygen

# Output:
# Generating new WRAITH identity...
# Keypair saved to: /home/user/.config/wraith/keypair.secret
# Your Node ID: a1b2c3d4e5f67890abcdef0123456789abcdef0123456789abcdef0123456789
```

**2. View Your Public Key:**
```bash
# Display your public key (share this with peers)
wraith keygen --show-public

# Output:
# Node ID (Ed25519 public key):
# a1b2c3d4e5f67890abcdef0123456789abcdef0123456789abcdef0123456789
#
# Share this key with peers to allow them to send you files.
```

**3. Initialize Configuration:**
```bash
# Generate default configuration
wraith config init

# Configuration saved to: ~/.config/wraith/config.toml
```

**4. Verify Setup:**
```bash
# Check configuration
wraith config show

# Output should show your node configuration
```

### 1.5 Quick Start: Send Your First File

Let's send a file to another WRAITH node:

**Scenario:** Alice wants to send a file to Bob.

**Bob's Setup (Receiver):**
```bash
# Bob starts the daemon to receive files
wraith daemon --output ~/Downloads/wraith

# Output:
# WRAITH daemon started
# Node ID: bob_public_key_here...
# Listening on: 0.0.0.0:41641
# Output directory: /home/bob/Downloads/wraith
```

**Alice's Setup (Sender):**
```bash
# Alice sends a file to Bob
wraith send document.pdf --to bob_public_key_here...

# Output:
# Establishing session with peer...
# Handshake complete (Noise_XX)
# Transferring: document.pdf (2.5 MB)
# [========================================] 100% (2.5 MB / 2.5 MB)
# Transfer complete: 2.5 MB in 0.8s (3.1 MB/s)
# Verification: BLAKE3 hash matches
```

**Bob's View:**
```bash
# Bob sees the received file
# Received: document.pdf from alice_public_key_here...
# Size: 2.5 MB
# Hash: verified
# Saved to: /home/bob/Downloads/wraith/document.pdf
```

Congratulations! You've just completed your first secure file transfer with WRAITH Protocol.

---

## 2. Basic Usage

### 2.1 Sending Files

The `send` command transfers files to a peer node.

**Basic Send:**
```bash
wraith send <FILE> --to <PEER_ID>
```

**Examples:**

1. **Send a single file:**
```bash
wraith send report.pdf --to a1b2c3d4e5f6...
```

2. **Send with progress display:**
```bash
wraith send large_file.zip --to a1b2c3d4e5f6... --progress

# Output:
# Sending: large_file.zip (1.5 GB)
# [=========>                    ] 34% (510 MB / 1.5 GB)
# Speed: 45.2 MB/s | ETA: 22s | Chunks: 2040/6000
```

3. **Send with custom chunk size:**
```bash
wraith send huge_file.iso --to a1b2c3d4e5f6... --chunk-size 1048576

# Chunk size: 1 MB (instead of default 256 KB)
# Larger chunks = fewer round trips but less granular resume
```

4. **Send with obfuscation:**
```bash
wraith send confidential.zip --to a1b2c3d4e5f6... --obfuscation high

# Traffic will use:
# - Size class padding
# - Timing jitter (0-100ms)
# - Cover traffic
# - Protocol header obfuscation
```

**Send Options:**
```
wraith send [OPTIONS] <FILE>

Options:
  -t, --to <PEER_ID>            Recipient's node ID (required)
  -o, --obfuscation <LEVEL>     Obfuscation: none, low, medium, high, paranoid
  -r, --resume                  Resume interrupted transfer
  -p, --progress                Show progress bar
  --chunk-size <BYTES>          Chunk size (default: 262144 = 256 KB)
  -c, --config <FILE>           Custom config file
  -v, --verbose                 Enable verbose output
```

### 2.2 Receiving Files

The `receive` command listens for incoming file transfers.

**Basic Receive:**
```bash
wraith receive --output <DIR>
```

**Examples:**

1. **Receive to specific directory:**
```bash
wraith receive --output ~/Downloads

# Files will be saved to: ~/Downloads/
```

2. **Run as daemon:**
```bash
wraith receive --daemon --output ~/Downloads

# Daemonizes and runs in background
# Use 'wraith status' to check status
```

3. **Enable multi-peer downloads:**
```bash
wraith receive --multi-peer --output ~/Downloads

# Will download chunks from multiple peers simultaneously
# Speeds up large file transfers significantly
```

4. **Auto-accept from trusted peers:**
```bash
wraith receive --auto-accept --output ~/Downloads

# Automatically accepts transfers from peers in trusted_peers.toml
```

**Receive Options:**
```
wraith receive [OPTIONS]

Options:
  -o, --output <DIR>            Output directory (default: current directory)
  -d, --daemon                  Run as background daemon
  --multi-peer                  Enable multi-peer downloads
  --auto-accept                 Auto-accept transfers from trusted peers
  -c, --config <FILE>           Custom config file
  -v, --verbose                 Enable verbose output
```

### 2.3 Running as a Daemon

For persistent operation, run WRAITH as a daemon:

**Start Daemon:**
```bash
# Basic daemon
wraith daemon --output ~/Downloads

# With custom config
wraith daemon --config ~/.config/wraith/custom.toml

# Foreground mode (for debugging)
wraith daemon --foreground
```

**Check Daemon Status:**
```bash
wraith status

# Output:
# WRAITH Node Status
# ==================
# Node ID:      a1b2c3d4e5f6...
# Uptime:       2h 34m 12s
# Network:      UDP (NAT: Full Cone)
#
# Active Transfers (2):
#   [SEND] report.pdf -> peer1... (78% complete, 12.3 MB/s)
#   [RECV] video.mp4 <- peer2... (45% complete, 8.7 MB/s)
#
# Connected Peers: 5
# DHT Nodes: 42
```

**Stop Daemon:**
```bash
# Graceful shutdown
wraith daemon --stop

# Force stop (if graceful fails)
pkill -9 wraith
```

### 2.4 Managing Peers

**List Known Peers:**
```bash
wraith peers

# Output:
# Known Peers (15):
#   peer1... (192.168.1.100:41641) - last seen: 5m ago
#   peer2... (203.0.113.50:41641) - last seen: 1h ago
#   ...
```

**Add Peer Manually:**
```bash
# Add peer with IP:port
wraith peers --add <NODE_ID>@<IP>:<PORT>

# Example:
wraith peers --add a1b2c3d4...@192.168.1.100:41641
```

**Remove Peer:**
```bash
wraith peers --remove <NODE_ID>

# Example:
wraith peers --remove a1b2c3d4...
```

**Peer Discovery via DHT:**
```bash
# Search for peers in DHT
wraith peers --discover

# This queries the DHT for peers sharing files
# Results are automatically cached
```

### 2.5 Transfer Status and Management

**Check Active Transfers:**
```bash
wraith status --transfers

# Output:
# Active Transfers:
#
#   [SEND] report.pdf
#   Peer: peer1... (192.168.1.100:41641)
#   Progress: 78% (3.9 MB / 5.0 MB)
#   Speed: 12.3 MB/s
#   ETA: 8s
#   Chunks: 15/20 completed
#
#   [RECV] video.mp4
#   Peer: peer2... (203.0.113.50:41641)
#   Progress: 45% (450 MB / 1000 MB)
#   Speed: 8.7 MB/s
#   ETA: 1m 3s
#   Chunks: 1758/3906 completed
#   Multi-peer: 3 sources
```

**Cancel Transfer:**
```bash
# Cancel by transfer ID
wraith transfer cancel <TRANSFER_ID>

# Cancel all transfers
wraith transfer cancel --all
```

**Pause/Resume Transfer:**
```bash
# Pause transfer
wraith transfer pause <TRANSFER_ID>

# Resume transfer
wraith transfer resume <TRANSFER_ID>
```

### 2.6 Configuration Management

**Show Effective Configuration:**
```bash
# Display configuration with defaults
wraith config show

# Show specific section
wraith config show --section network
wraith config show --section obfuscation
```

**Validate Configuration:**
```bash
# Check configuration syntax
wraith config validate

# Validate specific file
wraith config validate --file /path/to/config.toml
```

**Generate Default Configuration:**
```bash
# Create default config
wraith config init --output ~/.config/wraith/config.toml

# Force overwrite existing
wraith config init --force
```

---

## 3. Configuration

### 3.1 Configuration File Format

WRAITH uses TOML format for configuration. The default location is `~/.config/wraith/config.toml`.

**Basic Configuration Structure:**
```toml
[node]
# Node identity
public_key = "auto-generated"
private_key_file = "~/.config/wraith/keypair.secret"

[network]
# Network settings
listen_addr = "0.0.0.0:41641"

[obfuscation]
# Obfuscation settings
default_level = "medium"

[discovery]
# Peer discovery
dht_enabled = true

[transfer]
# File transfer settings
chunk_size = 262144  # 256 KB
output_dir = "~/Downloads/wraith"

[logging]
# Logging configuration
level = "info"
```

### 3.2 Node Identity Configuration

Configure your node's identity and network presence:

```toml
[node]
# Public key (auto-generated, do not modify)
public_key = "a1b2c3d4e5f67890..."

# Path to private key file (must be chmod 600)
private_key_file = "~/.config/wraith/keypair.secret"

# Optional: Node nickname for display purposes
# Max 64 characters, alphanumeric + underscore
nickname = "alice_workstation"
```

**Security Notes:**
- Never share `private_key_file` or modify it manually
- Keep permissions restrictive: `chmod 600 keypair.secret`
- Back up your keypair to a secure location

### 3.3 Network Settings

Configure how your node connects to the network:

```toml
[network]
# Address to listen on
# "0.0.0.0" = all interfaces
# "127.0.0.1" = localhost only
listen_addr = "0.0.0.0:41641"

# Optional: Override auto-detected public IP
# Required if behind NAT with manual port forwarding
# public_addr = "203.0.113.50:41641"

# Enable UPnP automatic port mapping
enable_upnp = true

# Enable NAT-PMP port mapping
enable_nat_pmp = true

# Idle connection timeout
idle_timeout = "30s"

# Handshake timeout
handshake_timeout = "10s"

# Maximum concurrent connections
max_connections = 1000

# Connection rate limit (per second)
connection_rate_limit = 100
```

**Common Scenarios:**

1. **Behind NAT with UPnP:**
```toml
[network]
listen_addr = "0.0.0.0:41641"
enable_upnp = true
```

2. **Behind NAT with manual port forwarding:**
```toml
[network]
listen_addr = "0.0.0.0:41641"
public_addr = "your.public.ip:41641"
enable_upnp = false
```

3. **Public server:**
```toml
[network]
listen_addr = "0.0.0.0:41641"
public_addr = "server.example.com:41641"
enable_upnp = false
max_connections = 10000
```

### 3.4 Transport Configuration

Configure the transport layer (UDP vs. AF_XDP):

```toml
[transport]
# Transport mode: "auto", "udp", "af-xdp"
# - auto: Try AF_XDP, fall back to UDP
# - udp: Force UDP (works everywhere)
# - af-xdp: Force AF_XDP (Linux only, requires CAP_NET_RAW)
mode = "auto"

# Network interface for AF_XDP
# Only used if mode = "af-xdp"
interface = "eth0"

# Maximum packet size (bytes)
# Default: 1472 (fits in 1500 MTU)
# Jumbo frames: 8972 (9000 MTU)
max_packet_size = 1472

# Socket buffer sizes (bytes)
send_buffer_size = 2097152  # 2 MB
recv_buffer_size = 2097152  # 2 MB
```

**AF_XDP Configuration (Advanced):**
```toml
[transport]
mode = "af-xdp"
interface = "eth0"
max_packet_size = 1472

# AF_XDP UMEM size (shared memory for zero-copy)
xdp_umem_size = 16777216  # 16 MB

# Number of XDP ring entries
xdp_ring_size = 4096

# Enable busy polling (reduces latency, increases CPU)
busy_poll = false
busy_poll_timeout = 50
```

**When to Use AF_XDP:**
- Linux 6.2+ with compatible NIC
- High-throughput scenarios (>1 Gbps)
- Low-latency requirements (<1ms)
- Root access or CAP_NET_RAW capability available

### 3.5 Obfuscation Settings

Configure traffic obfuscation to evade deep packet inspection:

```toml
[obfuscation]
# Default obfuscation level for all transfers
# Levels: none, low, medium, high, paranoid
default_level = "medium"

# Enable TLS 1.3 mimicry (looks like HTTPS)
tls_mimicry = false
tls_server_name = "cloudflare.com"

# Padding mode: "random", "size_class", "constant"
padding_mode = "size_class"

# Size classes for padding (bytes)
size_classes = [64, 128, 256, 512, 1024, 1472]

# Enable timing jitter
timing_jitter = true
jitter_min = 0    # milliseconds
jitter_max = 50   # milliseconds

# Enable cover traffic (dummy packets)
cover_traffic = false
cover_traffic_rate = 10  # packets per second
```

**Obfuscation Levels:**

| Level | Padding | Timing | Cover Traffic | Protocol Mimicry | Throughput Impact |
|-------|---------|--------|---------------|------------------|-------------------|
| **none** | No | No | No | No | 0% |
| **low** | Size classes | No | No | No | ~5% |
| **medium** | Size classes | Jitter 0-50ms | No | No | ~10% |
| **high** | Size classes | Jitter 0-100ms | Yes | Header obfuscation | ~30% |
| **paranoid** | Constant rate | Fixed intervals | Yes | Full TLS mimicry | ~60% |

**Example Configurations:**

1. **Maximum Privacy (Paranoid):**
```toml
[obfuscation]
default_level = "paranoid"
tls_mimicry = true
tls_server_name = "cloudflare.com"
padding_mode = "constant"
timing_jitter = true
jitter_min = 10
jitter_max = 100
cover_traffic = true
cover_traffic_rate = 5
constant_rate = true
constant_rate_bps = 500000  # 500 KB/s
```

2. **Balanced Privacy/Performance:**
```toml
[obfuscation]
default_level = "medium"
padding_mode = "size_class"
timing_jitter = true
jitter_min = 0
jitter_max = 50
```

3. **No Obfuscation (Maximum Performance):**
```toml
[obfuscation]
default_level = "none"
padding_mode = "random"
padding_min = 0
padding_max = 0
timing_jitter = false
```

### 3.6 Discovery Configuration

Configure peer discovery and DHT:

```toml
[discovery]
# Enable Kademlia DHT discovery
dht_enabled = true

# DHT bootstrap nodes
bootstrap_nodes = [
    "bootstrap1.wraith.network:41641",
    "bootstrap2.wraith.network:41641",
    "bootstrap3.wraith.network:41641",
]

# DHT replication factor (k)
replication_factor = 20

# DHT lookup concurrency (alpha)
query_concurrency = 3

# DHT bucket refresh interval
refresh_interval = "1h"

# DHT announce interval
announce_interval = "30m"

# Enable relay discovery
relay_enabled = true

# Relay servers (for NAT traversal fallback)
relay_servers = [
    "relay1.wraith.network:41641",
    "relay2.wraith.network:41641",
]

# Enable local discovery (mDNS/DNS-SD)
local_discovery = true

# Enable IPv6
ipv6_enabled = true
```

**Discovery Scenarios:**

1. **Public DHT with Relay Fallback:**
```toml
[discovery]
dht_enabled = true
relay_enabled = true
local_discovery = false
```

2. **Private Network (LAN-only):**
```toml
[discovery]
dht_enabled = false
relay_enabled = false
local_discovery = true
```

3. **Manual Peer Management:**
```toml
[discovery]
dht_enabled = false
relay_enabled = false
local_discovery = false

# Add peers manually via CLI:
# wraith peers --add <NODE_ID>@<IP>:<PORT>
```

### 3.7 File Transfer Settings

Configure file transfer behavior:

```toml
[transfer]
# Default chunk size (bytes)
# Larger = more efficient, smaller = better resume granularity
chunk_size = 262144  # 256 KB

# Maximum parallel chunks per transfer
max_parallel_chunks = 16

# Verify each chunk (integrity check)
verify_chunks = true

# Output directory for received files
output_dir = "~/Downloads/wraith"

# Auto-accept transfers from known peers
auto_accept = false

# Auto-accept threshold (successful transfers required)
auto_accept_threshold = 3

# Maximum file size (0 = unlimited)
max_file_size = 0

# Enable resume for interrupted transfers
resume_enabled = true

# Resume metadata directory
resume_dir = "~/.local/share/wraith/resume"

# Resume metadata TTL (delete after this duration)
resume_ttl = "7d"

# Enable multi-peer downloads
multi_peer = true

# Maximum peers per download
max_peers_per_download = 5

# Peer request timeout
peer_timeout = "30s"
```

**Transfer Optimization:**

1. **High-Bandwidth LAN (10 Gbps):**
```toml
[transfer]
chunk_size = 1048576  # 1 MB chunks
max_parallel_chunks = 32
verify_chunks = true
multi_peer = false  # Single peer is fast enough
```

2. **Slow Internet (100 Mbps):**
```toml
[transfer]
chunk_size = 131072  # 128 KB chunks
max_parallel_chunks = 8
verify_chunks = true
multi_peer = true
max_peers_per_download = 3
```

3. **Satellite/High-Latency (500ms RTT):**
```toml
[transfer]
chunk_size = 524288  # 512 KB chunks
max_parallel_chunks = 64  # High parallelism
verify_chunks = true
```

### 3.8 Logging Configuration

Configure logging verbosity and output:

```toml
[logging]
# Log level: "trace", "debug", "info", "warn", "error"
level = "info"

# Log format: "text", "json"
format = "text"

# Log output: "stdout", "stderr", or file path
output = "stdout"

# Maximum log file size (with rotation)
max_size = "100MB"

# Maximum rotated log files to keep
max_files = 10

# Compress rotated logs
compress = true

# Include timestamps
timestamps = true

# Include thread IDs
thread_ids = false

# Include file/line numbers
file_line = false

# Enable audit logging
audit_enabled = false

# Audit log file
audit_file = "/var/log/wraith/audit.log"

# Audit events to log
audit_events = [
    "handshake_complete",
    "transfer_start",
    "transfer_complete",
    "peer_connected",
    "peer_disconnected",
]
```

**Logging Scenarios:**

1. **Production Server:**
```toml
[logging]
level = "warn"
format = "json"
output = "/var/log/wraith/wraith.log"
max_size = "100MB"
max_files = 30
compress = true
audit_enabled = true
```

2. **Development/Debugging:**
```toml
[logging]
level = "debug"
format = "text"
output = "stdout"
timestamps = true
thread_ids = true
file_line = true
```

3. **Minimal Logging (High Performance):**
```toml
[logging]
level = "error"
format = "text"
output = "stderr"
timestamps = false
```

### 3.9 Environment Variables

Override configuration with environment variables:

```bash
# Override log level
export WRAITH_LOG_LEVEL=debug
wraith daemon

# Override listen address
export WRAITH_LISTEN_ADDR=0.0.0.0:50000
wraith daemon

# Override output directory
export WRAITH_OUTPUT_DIR=~/my-downloads
wraith receive

# Override config file location
export WRAITH_CONFIG=~/custom-config.toml
wraith daemon
```

**All Environment Variables:**

| Variable | Description | Example |
|----------|-------------|---------|
| `WRAITH_CONFIG` | Config file path | `~/.config/wraith/config.toml` |
| `WRAITH_PRIVATE_KEY_FILE` | Private key path | `~/.config/wraith/keypair.secret` |
| `WRAITH_LISTEN_ADDR` | Listen address | `0.0.0.0:41641` |
| `WRAITH_PUBLIC_ADDR` | Public address | `203.0.113.50:41641` |
| `WRAITH_TRANSPORT_MODE` | Transport mode | `auto`, `udp`, `af-xdp` |
| `WRAITH_INTERFACE` | Network interface | `eth0` |
| `WRAITH_LOG_LEVEL` | Log level | `trace`, `debug`, `info`, `warn`, `error` |
| `WRAITH_OUTPUT_DIR` | Download directory | `~/Downloads/wraith` |

---

## 4. Advanced Topics

### 4.1 Multi-Peer Downloads

Multi-peer downloads allow downloading files from multiple sources simultaneously, significantly speeding up transfers.

**How It Works:**
1. File is split into chunks (default: 256 KB)
2. Different chunks are requested from different peers
3. Chunks are downloaded in parallel
4. Chunks are reassembled and verified

**Enable Multi-Peer:**
```bash
# Start receiver with multi-peer enabled
wraith receive --multi-peer --output ~/Downloads
```

**Configuration:**
```toml
[transfer]
multi_peer = true
max_peers_per_download = 5
chunk_size = 262144
max_parallel_chunks = 16
```

**Performance Expectations:**

| Number of Peers | Expected Speedup |
|-----------------|------------------|
| 1 | 1x (baseline) |
| 2 | ~1.9x |
| 3 | ~2.8x |
| 4 | ~3.7x |
| 5 | ~4.5x |

**Chunk Assignment Strategies:**

WRAITH supports multiple chunk assignment strategies:

1. **RoundRobin (Default):**
   - Equal distribution across all peers
   - Fair but may not be optimal
   ```toml
   [transfer]
   multi_peer_strategy = "round_robin"
   ```

2. **FastestFirst:**
   - Prioritizes peers with highest throughput
   - Optimal for heterogeneous peer speeds
   ```toml
   [transfer]
   multi_peer_strategy = "fastest_first"
   ```

3. **Geographic:**
   - Prefers peers with lowest RTT (latency)
   - Optimal for latency-sensitive transfers
   ```toml
   [transfer]
   multi_peer_strategy = "geographic"
   ```

4. **Adaptive:**
   - Dynamic strategy based on performance score
   - Balances reliability (40%), speed (40%), latency (20%)
   ```toml
   [transfer]
   multi_peer_strategy = "adaptive"
   ```

**Monitoring Multi-Peer Transfers:**
```bash
wraith status --transfers

# Output shows per-peer statistics:
# [RECV] large_file.iso
# Progress: 67% (6.7 GB / 10 GB)
# Peers (3):
#   peer1... (192.168.1.100) - 45.2 MB/s (2500 chunks)
#   peer2... (192.168.1.101) - 38.7 MB/s (2100 chunks)
#   peer3... (192.168.1.102) - 29.3 MB/s (1600 chunks)
# Combined speed: 113.2 MB/s
# ETA: 30s
```

### 4.2 Resume After Interruption

WRAITH supports automatic resume for interrupted transfers.

**How Resume Works:**
1. Transfer state is persisted to disk every N chunks
2. If transfer is interrupted, state is loaded on restart
3. Only missing chunks are re-downloaded
4. BLAKE3 tree hash ensures integrity

**Enable Resume:**
```toml
[transfer]
resume_enabled = true
resume_dir = "~/.local/share/wraith/resume"
resume_ttl = "7d"  # Delete old state after 7 days
```

**Resume Usage:**
```bash
# Send file (transfer interrupted)
wraith send large_file.iso --to peer...
# ... connection lost ...

# Resume transfer (automatic)
wraith send large_file.iso --to peer...
# Output: Resuming transfer (3200/6000 chunks completed)
```

**Resume State Format:**
The resume state is stored as JSON:
```json
{
  "transfer_id": "unique_transfer_id",
  "peer_id": "peer_node_id",
  "file_hash": "blake3_root_hash",
  "file_size": 1572864000,
  "chunk_size": 262144,
  "total_chunks": 6000,
  "completed_chunks": [0, 1, 2, ..., 3199],
  "started_at": "2025-12-05T10:00:00Z",
  "updated_at": "2025-12-05T10:15:30Z"
}
```

**Manual Resume Management:**
```bash
# List resume states
wraith transfer list-resume

# Clean up old resume states
wraith transfer clean-resume --older-than 7d

# Delete specific resume state
wraith transfer delete-resume <TRANSFER_ID>
```

### 4.3 NAT Traversal Setup

WRAITH supports multiple NAT traversal techniques:

**NAT Types:**
1. **Full Cone:** Any external host can send packets to internal host
2. **Restricted Cone:** Only hosts that have received packets can reply
3. **Port-Restricted Cone:** Only specific ports can reply
4. **Symmetric:** Most restrictive, different mapping for each destination

**Detect NAT Type:**
```bash
wraith status --network

# Output:
# Network Status:
# NAT Type: Port-Restricted Cone
# Public Address: 203.0.113.50:41641
# UPnP: Enabled (mapping successful)
```

**NAT Traversal Strategies:**

1. **UPnP/NAT-PMP (Automatic):**
```toml
[network]
enable_upnp = true
enable_nat_pmp = true
```

2. **STUN + UDP Hole Punching:**
```toml
[discovery]
relay_enabled = true
relay_servers = [
    "stun.wraith.network:41641",
]
```

3. **Manual Port Forwarding:**
```toml
[network]
listen_addr = "0.0.0.0:41641"
public_addr = "your.public.ip:41641"
enable_upnp = false
```

4. **Relay Fallback (Last Resort):**
```toml
[discovery]
relay_enabled = true
relay_servers = [
    "relay1.wraith.network:41641",
    "relay2.wraith.network:41641",
]
```

**Troubleshooting NAT:**
```bash
# Test STUN connectivity
wraith network test-stun

# Test UPnP mapping
wraith network test-upnp

# Test relay connectivity
wraith network test-relay
```

### 4.4 Relay Server Configuration

Run your own relay server for NAT traversal:

**1. Install WRAITH on Server:**
```bash
# Public VPS with no NAT
# Static IP: relay.example.com
```

**2. Configure Relay Mode:**
```toml
[node]
nickname = "relay-server-us-east-1"

[network]
listen_addr = "0.0.0.0:41641"
public_addr = "relay.example.com:41641"
enable_upnp = false
max_connections = 50000

[discovery]
relay_enabled = false  # This IS a relay, don't use other relays
relay_mode = true      # Enable relay functionality

[session]
max_sessions = 50000
idle_timeout = "120s"
```

**3. Start Relay Server:**
```bash
wraith relay --config /etc/wraith/relay.toml

# Or as systemd service:
sudo systemctl start wraith-relay
sudo systemctl enable wraith-relay
```

**4. Configure Clients to Use Your Relay:**
```toml
[discovery]
relay_enabled = true
relay_servers = [
    "relay.example.com:41641",
]
```

**Relay Server Requirements:**
- Public IP address (no NAT)
- Open UDP port 41641
- High bandwidth (relay traffic passes through)
- Low latency for good performance

### 4.5 DHT Bootstrap Nodes

Run your own DHT bootstrap node:

**1. Configure Bootstrap Node:**
```toml
[node]
nickname = "bootstrap-node-us-west-1"

[network]
listen_addr = "0.0.0.0:41641"
public_addr = "bootstrap.example.com:41641"
max_connections = 100000

[discovery]
dht_enabled = true
bootstrap_mode = true  # This is a bootstrap node
bootstrap_nodes = []   # Bootstrap nodes don't bootstrap from others
```

**2. Start Bootstrap Node:**
```bash
wraith daemon --config /etc/wraith/bootstrap.toml
```

**3. Configure Clients:**
```toml
[discovery]
dht_enabled = true
bootstrap_nodes = [
    "bootstrap.example.com:41641",
]
```

**Bootstrap Node Requirements:**
- Public IP with open UDP port
- High uptime (99%+)
- Sufficient bandwidth for DHT queries
- 4+ GB RAM for routing table

### 4.6 Key Rotation and Security

**Rotating Your Keypair:**
```bash
# Backup old keypair
cp ~/.config/wraith/keypair.secret ~/.config/wraith/keypair.secret.backup

# Generate new keypair
wraith keygen --force

# Output:
# WARNING: This will replace your existing keypair!
# Your node ID will change and peers will need your new public key.
# Proceed? [y/N]: y
#
# New keypair generated.
# New Node ID: f1e2d3c4b5a69807...
```

**Passphrase-Protected Keys:**
```bash
# Generate keypair with passphrase encryption
wraith keygen --passphrase

# You'll be prompted for a passphrase
# The private key will be encrypted at rest using Argon2id + XChaCha20-Poly1305
```

**Key Security Best Practices:**
1. Set restrictive permissions: `chmod 600 keypair.secret`
2. Back up to encrypted storage
3. Use passphrase encryption for sensitive environments
4. Rotate keys periodically (every 6-12 months)
5. Revoke old keys when rotated

### 4.7 Performance Tuning

**High-Throughput Tuning (10 Gbps+):**
```toml
[transport]
mode = "af-xdp"
interface = "eth0"
xdp_umem_size = 67108864  # 64 MB
xdp_ring_size = 8192
busy_poll = true

[session]
max_sessions = 10000
max_retransmissions = 3
rto_initial = "100ms"

[congestion]
algorithm = "bbr"
max_cwnd = 50000

[transfer]
chunk_size = 1048576  # 1 MB
max_parallel_chunks = 32
verify_chunks = true

[files]
backend = "io_uring"
ring_size = 4096
io_polling = true
direct_io = true
```

**Low-Latency Tuning (<1ms):**
```toml
[transport]
mode = "af-xdp"
busy_poll = true
busy_poll_timeout = 50

[session]
rto_initial = "50ms"
rto_min = "10ms"
keepalive_interval = "5s"

[congestion]
algorithm = "bbr"
```

**Memory-Constrained Tuning:**
```toml
[session]
max_sessions = 100

[transfer]
chunk_size = 131072  # 128 KB
max_parallel_chunks = 4

[files]
backend = "standard"
read_buffer_size = 262144
write_buffer_size = 262144
```

---

## 5. Security Best Practices

### 5.1 Key Management

**Protect Your Private Key:**
```bash
# Set restrictive permissions
chmod 600 ~/.config/wraith/keypair.secret

# Verify permissions
ls -l ~/.config/wraith/keypair.secret
# Should show: -rw------- (600)
```

**Backup Your Keypair:**
```bash
# Encrypted backup
tar czf wraith-keypair-backup.tar.gz -C ~/.config/wraith keypair.secret
gpg --encrypt --recipient your@email.com wraith-keypair-backup.tar.gz
rm wraith-keypair-backup.tar.gz

# Store encrypted backup securely
mv wraith-keypair-backup.tar.gz.gpg ~/Backups/
```

**Passphrase Protection:**
```bash
# Generate keypair with passphrase
wraith keygen --passphrase

# Change passphrase
wraith keygen --change-passphrase
```

### 5.2 Network Security

**Use Obfuscation on Untrusted Networks:**
```bash
# High obfuscation for public WiFi
wraith send file.zip --to peer... --obfuscation high

# Paranoid mode for maximum privacy
wraith send sensitive.pdf --to peer... --obfuscation paranoid
```

**Verify Peer Public Keys Out-of-Band:**
```bash
# Share public keys via secure channel
# Signal, in-person, PGP email, etc.

# Alice sends her public key to Bob via Signal:
wraith keygen --show-public | pbcopy

# Bob verifies and adds Alice manually:
wraith peers --add <ALICE_KEY>@192.168.1.100:41641
```

**Monitor Active Connections:**
```bash
# Check connected peers
wraith status --peers

# Disconnect suspicious peer
wraith peers --disconnect <PEER_ID>

# Block peer permanently
wraith peers --block <PEER_ID>
```

### 5.3 Operational Security

**Run Daemon with Least Privilege:**
```bash
# Create dedicated user
sudo useradd -r -s /bin/false wraith

# Set ownership
sudo chown -R wraith:wraith /var/lib/wraith
sudo chown -R wraith:wraith /var/log/wraith

# Run as wraith user
sudo -u wraith wraith daemon --config /etc/wraith/config.toml
```

**Enable Logging for Auditing:**
```toml
[logging]
level = "info"
output = "/var/log/wraith/wraith.log"
audit_enabled = true
audit_file = "/var/log/wraith/audit.log"
audit_events = [
    "handshake_complete",
    "transfer_start",
    "transfer_complete",
    "authentication_failure",
]
```

**Regular Updates:**
```bash
# Check current version
wraith --version

# Check for updates
curl -s https://api.github.com/repos/doublegate/WRAITH-Protocol/releases/latest | jq -r .tag_name

# Update via package manager
sudo apt update && sudo apt upgrade wraith  # Debian/Ubuntu
sudo dnf upgrade wraith                      # Fedora/RHEL
```

### 5.4 Trusted Peers Management

**Configure Trusted Peers:**
```toml
# ~/.config/wraith/trusted_peers.toml
[[peers]]
node_id = "a1b2c3d4e5f67890..."
nickname = "alice"
added_at = "2025-12-05T10:00:00Z"
trust_level = "high"  # high, medium, low

[[peers]]
node_id = "f1e2d3c4b5a69807..."
nickname = "bob"
added_at = "2025-12-05T11:30:00Z"
trust_level = "high"
```

**Auto-Accept from Trusted Peers:**
```toml
[transfer]
auto_accept = true
auto_accept_threshold = 3  # Require 3 successful transfers before auto-accept
```

**Blocked Peers:**
```bash
# Block peer
wraith peers --block <PEER_ID>

# Unblock peer
wraith peers --unblock <PEER_ID>

# List blocked peers
wraith peers --list-blocked
```

### 5.5 Firewall Configuration

**Allow WRAITH Traffic:**

**UFW (Ubuntu):**
```bash
sudo ufw allow 41641/udp comment 'WRAITH Protocol'
sudo ufw reload
```

**firewalld (RHEL/Fedora):**
```bash
sudo firewall-cmd --permanent --add-port=41641/udp
sudo firewall-cmd --reload
```

**iptables (Generic):**
```bash
sudo iptables -A INPUT -p udp --dport 41641 -j ACCEPT
sudo iptables-save > /etc/iptables/rules.v4
```

**Restrict to Specific IPs:**
```bash
# Allow only from trusted subnet
sudo ufw allow from 192.168.1.0/24 to any port 41641 proto udp
```

---

## 6. Examples

### 6.1 Simple File Transfer

**Scenario:** Alice wants to send a document to Bob on the same LAN.

**Bob (Receiver):**
```bash
# Start receiver
wraith receive --output ~/Downloads

# Output:
# WRAITH receiver started
# Node ID: bob_123...
# Listening on: 0.0.0.0:41641
# Output directory: /home/bob/Downloads
```

**Alice (Sender):**
```bash
# Send file to Bob
wraith send report.pdf --to bob_123... --progress

# Output:
# Establishing session with bob_123...
# Handshake complete (Noise_XX, 48ms)
# Sending: report.pdf (5.2 MB)
# [========================================] 100% (5.2 MB / 5.2 MB)
# Speed: 125.3 MB/s | Time: 0.04s
# Verification: BLAKE3 hash verified
# Transfer complete
```

### 6.2 High-Security Transfer

**Scenario:** Alice needs to send confidential data over public internet with maximum security.

**Alice's Configuration:**
```toml
[obfuscation]
default_level = "paranoid"
tls_mimicry = true
tls_server_name = "www.cloudflare.com"
padding_mode = "constant"
timing_jitter = true
jitter_min = 10
jitter_max = 100
cover_traffic = true
constant_rate = true
constant_rate_bps = 500000  # 500 KB/s
```

**Transfer:**
```bash
# Send with maximum obfuscation
wraith send confidential.pdf --to bob_123... --obfuscation paranoid --progress

# Output:
# Obfuscation: PARANOID mode
#   - TLS 1.3 mimicry enabled
#   - Constant-rate padding (500 KB/s)
#   - Timing jitter: 10-100ms
#   - Cover traffic: 5 pkt/s
#
# Establishing session with bob_123...
# Handshake complete (Noise_XX, 152ms)
# Sending: confidential.pdf (10 MB)
# [========================================] 100% (10 MB / 10 MB)
# Speed: 450 KB/s (limited by obfuscation) | Time: 22.7s
# Verification: BLAKE3 hash verified
# Transfer complete
```

### 6.3 Multi-Peer Download

**Scenario:** Alice wants to download a large file from multiple peers for maximum speed.

**Setup:**
- peer1, peer2, peer3 all have the same file
- Alice wants to download from all three simultaneously

**Alice's Configuration:**
```toml
[transfer]
multi_peer = true
max_peers_per_download = 5
chunk_size = 262144
max_parallel_chunks = 32
multi_peer_strategy = "adaptive"
```

**Download:**
```bash
# Start multi-peer receiver
wraith receive --multi-peer --output ~/Downloads

# WRAITH will automatically:
# 1. Discover all peers with the file via DHT
# 2. Establish sessions with multiple peers
# 3. Assign chunks to peers dynamically
# 4. Download chunks in parallel
# 5. Reassemble and verify

# Output:
# [RECV] large_video.mp4 (2.5 GB)
# Peers discovered: 3 (peer1, peer2, peer3)
# Establishing sessions...
#
# Downloading from 3 peers:
#   peer1 (192.168.1.100): 85.3 MB/s (chunk 0-800)
#   peer2 (192.168.1.101): 92.1 MB/s (chunk 801-1600)
#   peer3 (192.168.1.102): 78.6 MB/s (chunk 1601-2400)
#
# [========================================] 100% (2.5 GB / 2.5 GB)
# Combined speed: 256.0 MB/s | Time: 9.8s
# Verification: BLAKE3 tree hash verified
# Transfer complete
```

### 6.4 Resume After Interruption

**Scenario:** Bob is downloading a large file but loses connection midway. He resumes the transfer.

**Initial Transfer:**
```bash
# Bob starts download
wraith receive --output ~/Downloads

# Transfer in progress...
# [=========>                    ] 34% (3.4 GB / 10 GB)
# ... connection lost ...
```

**Resume:**
```bash
# Bob restarts receiver
wraith receive --output ~/Downloads

# Output:
# Detected incomplete transfer: large_file.iso
# Transfer ID: transfer_abc123
# Progress: 34% (3400/10000 chunks completed)
# Resuming...
#
# Establishing session with peer...
# Sending resume bitmap (10000 bits = 1250 bytes)
# Peer confirmed, resuming from chunk 3401
#
# [=========>                    ] 34% (3.4 GB / 10 GB)
# ... transfer continues ...
# [========================================] 100% (10 GB / 10 GB)
# Speed: 95.2 MB/s | Time: 69.2s (total), 45.7s (resumed)
# Verification: BLAKE3 tree hash verified
# Transfer complete
```

### 6.5 Running as System Service

**Scenario:** Bob wants to run WRAITH as a system service for persistent operation.

**1. Create systemd service:**
```bash
sudo nano /etc/systemd/system/wraith.service
```

**2. Service configuration:**
```ini
[Unit]
Description=WRAITH Protocol Daemon
After=network.target
Documentation=https://github.com/doublegate/WRAITH-Protocol

[Service]
Type=simple
User=wraith
Group=wraith
ExecStart=/usr/local/bin/wraith daemon --config /etc/wraith/config.toml
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/wraith /var/log/wraith

[Install]
WantedBy=multi-user.target
```

**3. Enable and start:**
```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable wraith

# Start service
sudo systemctl start wraith

# Check status
sudo systemctl status wraith

# View logs
sudo journalctl -u wraith -f
```

### 6.6 Integration with Existing Applications

**Scenario:** Integrate WRAITH into a Rust application using the library API.

**Add Dependency:**
```toml
[dependencies]
wraith-core = "0.9"
tokio = { version = "1", features = ["full"] }
```

**Example Code:**
```rust
use wraith_core::node::{Node, NodeConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create node with random identity
    let config = NodeConfig::default();
    let node = Node::new_random(config)?;

    // Start node
    node.start().await?;
    println!("Node started: {}", node.id());

    // Send file to peer
    let peer_id = "a1b2c3d4e5f67890...".parse()?;
    let file_path = PathBuf::from("document.pdf");
    let transfer_id = node.send_file(peer_id, file_path).await?;

    // Wait for transfer to complete
    node.wait_for_transfer(transfer_id).await?;
    println!("Transfer complete!");

    // Stop node gracefully
    node.stop().await?;

    Ok(())
}
```

---

## Conclusion

This tutorial covered the essentials of using WRAITH Protocol, from installation and basic usage to advanced features like multi-peer downloads, NAT traversal, and security best practices.

**Next Steps:**
- Read the [Integration Guide](INTEGRATION_GUIDE.md) for library API details
- Review the [Configuration Reference](CONFIG_REFERENCE.md) for all settings
- Check the [Troubleshooting Guide](TROUBLESHOOTING.md) for common issues
- Join the [WRAITH Community](https://github.com/doublegate/WRAITH-Protocol/discussions)

**Additional Resources:**
- [Protocol Technical Details](../ref-docs/protocol_technical_details.md)
- [Security Model](architecture/security-model.md)
- [Performance Benchmarks](testing/performance-benchmarks.md)
- [API Reference](engineering/api-reference.md)

---

**WRAITH Protocol** - Secure, Fast, Invisible File Transfer

**Version:** 1.0.0 | **License:** MIT | **Language:** Rust 2024
