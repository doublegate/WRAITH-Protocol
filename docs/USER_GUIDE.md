# WRAITH Protocol User Guide

**Version:** 0.7.0
**Last Updated:** 2025-12-01
**Status:** Production Ready

---

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [CLI Commands](#cli-commands)
5. [Configuration](#configuration)
6. [File Transfer](#file-transfer)
7. [Obfuscation Modes](#obfuscation-modes)
8. [Multi-Peer Downloads](#multi-peer-downloads)
9. [Troubleshooting](#troubleshooting)
10. [FAQ](#faq)
11. [Security Best Practices](#security-best-practices)

---

## Introduction

WRAITH (Wire-speed Resilient Authenticated Invisible Transfer Handler) is a secure, high-performance peer-to-peer file transfer protocol designed for privacy and speed.

**Key Features:**

- **End-to-end encryption:** XChaCha20-Poly1305 AEAD with forward secrecy
- **Traffic obfuscation:** Multiple modes to evade deep packet inspection (DPI)
- **High performance:** Up to 10 Gbps with kernel bypass (AF_XDP)
- **Resume support:** Interrupted transfers resume automatically
- **Multi-peer downloads:** Download from multiple sources simultaneously
- **NAT traversal:** Works behind most firewalls and NAT devices
- **Decentralized:** DHT-based peer discovery, no central servers required

**Threat Model:**

WRAITH protects against:
- Passive network observers (ISPs, surveillance)
- Active man-in-the-middle attacks
- Traffic analysis (with obfuscation enabled)
- Deep packet inspection (protocol fingerprinting)

WRAITH does NOT provide:
- IP address anonymity (use Tor for that)
- Protection against compromised endpoints
- Deniability (peers know each other's public keys)

---

## Installation

### Linux (Recommended)

**Ubuntu/Debian:**
```bash
# Download latest release
wget https://github.com/doublegate/WRAITH-Protocol/releases/latest/download/wraith_amd64.deb

# Install
sudo dpkg -i wraith_amd64.deb

# Verify installation
wraith --version
```

**Fedora/RHEL:**
```bash
# Download RPM
wget https://github.com/doublegate/WRAITH-Protocol/releases/latest/download/wraith_x86_64.rpm

# Install
sudo rpm -i wraith_x86_64.rpm

# Verify
wraith --version
```

**Arch Linux (AUR):**
```bash
# Using yay
yay -S wraith-protocol

# Or using paru
paru -S wraith-protocol
```

### From Source (All Platforms)

**Requirements:**
- Rust 1.88+ (install via https://rustup.rs)
- Linux kernel 6.2+ (for AF_XDP support)
- Build tools: `build-essential`, `pkg-config`, `libssl-dev`

```bash
# Clone repository
git clone https://github.com/doublegate/WRAITH-Protocol
cd WRAITH-Protocol

# Build release binary
cargo build --release

# Install to system (optional)
sudo install -m 755 target/release/wraith /usr/local/bin/

# Verify
wraith --version
```

### macOS (Limited Support)

macOS support is limited to UDP transport (no AF_XDP kernel bypass).

```bash
# From source
git clone https://github.com/doublegate/WRAITH-Protocol
cd WRAITH-Protocol
cargo build --release --no-default-features --features udp-only

# Binary at target/release/wraith
```

---

## Quick Start

### 1. Generate Your Identity

First, create your WRAITH keypair:

```bash
# Generate new keypair (creates ~/.config/wraith/keypair.secret)
wraith keygen

# View your public key (share this with peers)
wraith keygen --show-public
```

**Output:**
```
Your WRAITH public key:
a1b2c3d4e5f6...0123456789ab

Share this key with peers to receive files.
```

### 2. Start the Daemon

Run WRAITH in background mode to receive files:

```bash
# Start daemon
wraith daemon

# Or with custom output directory
wraith daemon --output ~/Downloads/wraith
```

### 3. Send a File

To send a file to a peer:

```bash
# Basic send
wraith send document.pdf --to <peer-public-key>

# With progress display
wraith send large_file.zip --to <peer-public-key> --progress

# With high obfuscation
wraith send secret.txt --to <peer-public-key> --obfuscation paranoid
```

### 4. Receive Files

Files are automatically received when the daemon is running:

```bash
# Start receiving daemon
wraith receive --output ~/Downloads

# Files appear in output directory with verification status
```

---

## CLI Commands

### Global Options

```bash
wraith [OPTIONS] <COMMAND>

Options:
  -c, --config <FILE>    Path to config file (default: ~/.config/wraith/config.toml)
  -v, --verbose          Enable verbose output
  -q, --quiet            Suppress non-error output
  -h, --help             Print help
  -V, --version          Print version
```

### Commands

#### `wraith send`

Send a file to a recipient.

```bash
wraith send [OPTIONS] <FILE> --to <RECIPIENT>

Arguments:
  <FILE>                 Path to file to send

Options:
  -t, --to <KEY>         Recipient's public key (required)
  -o, --obfuscation <MODE>  Obfuscation level: none, low, medium, high, paranoid
  -r, --resume           Resume interrupted transfer
  -p, --progress         Show progress bar
  --chunk-size <SIZE>    Chunk size in bytes (default: 262144)
```

**Examples:**
```bash
# Send a file with default settings
wraith send report.pdf --to a1b2c3d4...

# Send with high obfuscation
wraith send confidential.zip --to a1b2c3d4... --obfuscation high

# Resume interrupted transfer
wraith send large_file.iso --to a1b2c3d4... --resume
```

#### `wraith receive`

Receive files from peers.

```bash
wraith receive [OPTIONS]

Options:
  -o, --output <DIR>     Output directory (default: current directory)
  -d, --daemon           Run as background daemon
  --multi-peer           Enable multi-peer downloads
  --auto-accept          Automatically accept all transfers
```

**Examples:**
```bash
# Receive to specific directory
wraith receive --output ~/Downloads

# Run as daemon
wraith receive --daemon --output ~/Downloads

# Enable multi-peer downloads
wraith receive --multi-peer --output ~/Downloads
```

#### `wraith daemon`

Run WRAITH as a background service.

```bash
wraith daemon [OPTIONS]

Options:
  -o, --output <DIR>     Output directory for received files
  --init                 Initialize new configuration
  --foreground           Run in foreground (don't daemonize)
```

**Examples:**
```bash
# Start daemon
wraith daemon

# Initialize config and start
wraith daemon --init

# Run in foreground (for debugging)
wraith daemon --foreground
```

#### `wraith status`

Show node status and active transfers.

```bash
wraith status [OPTIONS]

Options:
  --transfers            Show active transfers
  --peers                Show connected peers
  --network              Show network statistics
  --json                 Output as JSON
```

**Example output:**
```
WRAITH Node Status
==================
Node ID:      a1b2c3d4e5f6...
Uptime:       2h 34m 12s
Network:      UDP (NAT: symmetric)

Active Transfers (2):
  [SEND] report.pdf -> peer1... (78% complete, 12.3 MB/s)
  [RECV] video.mp4 <- peer2... (45% complete, 8.7 MB/s)

Connected Peers: 5
DHT Nodes: 42
```

#### `wraith peers`

List discovered peers.

```bash
wraith peers [OPTIONS]

Options:
  --add <KEY@ADDR>       Add peer manually (key@ip:port)
  --remove <KEY>         Remove peer
  --json                 Output as JSON
```

**Examples:**
```bash
# List all known peers
wraith peers

# Add peer manually
wraith peers --add a1b2c3...@192.168.1.100:41641

# Remove peer
wraith peers --remove a1b2c3...
```

#### `wraith keygen`

Generate or manage keypairs.

```bash
wraith keygen [OPTIONS]

Options:
  -o, --output <FILE>    Output file (default: ~/.config/wraith/keypair.secret)
  --show-public          Display public key
  --force                Overwrite existing keypair
```

**Examples:**
```bash
# Generate new keypair
wraith keygen

# Show existing public key
wraith keygen --show-public

# Force regenerate (WARNING: loses old identity)
wraith keygen --force
```

---

## Configuration

### Configuration File

Default location: `~/.config/wraith/config.toml`

```toml
# Node identity
[node]
# Auto-generated, do not modify
public_key = "a1b2c3d4e5f6789..."
private_key_file = "~/.config/wraith/keypair.secret"

# Network settings
[network]
listen_addr = "0.0.0.0:41641"
# Uncomment to override auto-detected public IP
# public_addr = "203.0.113.50:41641"

# Obfuscation settings
[obfuscation]
# Default obfuscation level: none, low, medium, high, paranoid
default_level = "medium"
# Enable TLS-mimicry mode (looks like HTTPS traffic)
tls_mimicry = false

# Peer discovery
[discovery]
# Enable DHT discovery
dht_enabled = true
# Bootstrap nodes (default: public bootstrap servers)
bootstrap_nodes = [
    "bootstrap1.wraith.network:41641",
    "bootstrap2.wraith.network:41641",
]
# Enable relay fallback for NAT traversal
relay_enabled = true

# File transfer settings
[transfer]
# Chunk size in bytes (default: 256 KB)
chunk_size = 262144
# Maximum parallel chunks
max_parallel_chunks = 16
# Output directory for received files
output_dir = "~/Downloads/wraith"
# Auto-accept transfers from known peers
auto_accept = false

# Logging
[logging]
# Log level: trace, debug, info, warn, error
level = "info"
# Log file (optional)
# file = "/var/log/wraith/wraith.log"
```

### Environment Variables

Override configuration with environment variables:

```bash
# Override listen address
WRAITH_LISTEN_ADDR="0.0.0.0:50000" wraith daemon

# Override log level
WRAITH_LOG_LEVEL="debug" wraith daemon

# Override output directory
WRAITH_OUTPUT_DIR="~/my-downloads" wraith receive
```

---

## File Transfer

### How Transfers Work

1. **Sender initiates:** Computes file tree hash (BLAKE3), chunks file into 256 KB pieces
2. **Handshake:** Noise_XX handshake establishes encrypted session
3. **Metadata exchange:** File size, chunk count, root hash sent
4. **Chunk transfer:** Chunks sent with integrity verification
5. **Completion:** Final hash verification ensures integrity

### Progress Tracking

```bash
# Enable progress display
wraith send large_file.zip --to <peer> --progress
```

**Progress output:**
```
Sending: large_file.zip (1.5 GB)
[=========>                    ] 34% (510 MB / 1.5 GB)
Speed: 45.2 MB/s | ETA: 22s | Chunks: 2040/6000
```

### Resume Support

Interrupted transfers automatically resume:

```bash
# If transfer is interrupted, just re-run
wraith send large_file.zip --to <peer>
# Automatically detects existing progress and resumes
```

### Integrity Verification

Every chunk is verified using BLAKE3 Merkle tree hashing:

- **Chunk hash:** Each 256 KB chunk has a BLAKE3 hash
- **Tree hash:** Merkle tree combines chunk hashes
- **Root hash:** Final root hash verifies entire file

```
Verification: Root hash verified (a1b2c3d4e5f6...)
Chunks verified: 6000/6000
File integrity: VERIFIED
```

---

## Obfuscation Modes

WRAITH supports multiple obfuscation levels to evade traffic analysis:

### None

No obfuscation. Fastest but easily identifiable.

```bash
wraith send file.txt --to <peer> --obfuscation none
```

### Low

Basic padding to hide packet sizes.

- Pads packets to size classes (64, 128, 256, 512, 1024 bytes)
- Minimal performance impact

```bash
wraith send file.txt --to <peer> --obfuscation low
```

### Medium (Default)

Padding + timing jitter.

- Size class padding
- Random timing delays (0-50ms)
- Good balance of privacy/performance

```bash
wraith send file.txt --to <peer> --obfuscation medium
```

### High

Full obfuscation suite.

- Size class padding
- Timing jitter (0-100ms)
- Cover traffic (dummy packets)
- Protocol header obfuscation

```bash
wraith send file.txt --to <peer> --obfuscation high
```

### Paranoid

Maximum obfuscation.

- All high-level features
- TLS mimicry (looks like HTTPS)
- Constant-rate transmission
- Full traffic analysis resistance

```bash
wraith send file.txt --to <peer> --obfuscation paranoid
```

**Warning:** Paranoid mode significantly reduces throughput (typically <10 MB/s).

---

## Multi-Peer Downloads

Download files from multiple sources simultaneously for faster transfers.

### Enabling Multi-Peer

```bash
# Start receiver with multi-peer enabled
wraith receive --multi-peer --output ~/Downloads
```

### How It Works

1. **Discovery:** DHT lookup finds multiple peers with the file
2. **Coordination:** Chunks assigned to different peers
3. **Parallel download:** Chunks downloaded simultaneously
4. **Reassembly:** Chunks combined and verified

### Performance

Multi-peer downloads scale nearly linearly:

| Peers | Expected Speedup |
|-------|------------------|
| 1     | 1x (baseline)    |
| 2     | ~1.9x            |
| 3     | ~2.8x            |
| 4     | ~3.7x            |
| 5     | ~4.5x            |

### Example

```bash
# Sender shares file
wraith send large_file.iso --to <peer>

# Multiple peers now have chunks

# Receiver downloads from all available peers
wraith receive --multi-peer --output ~/Downloads
# Output: Downloading from 3 peers... (combined: 120 MB/s)
```

---

## Troubleshooting

### Connection Issues

**Problem:** Cannot connect to peers

```bash
# Check NAT type
wraith status --network

# Output:
# NAT Type: symmetric (strict)
# Relay: enabled
```

**Solutions:**

1. **Symmetric NAT:** Use relay fallback (automatic)
   ```bash
   wraith daemon --relay-enabled
   ```

2. **Firewall blocking:** Open UDP port 41641
   ```bash
   sudo ufw allow 41641/udp
   ```

3. **Router issues:** Enable UPnP or manually forward port
   ```bash
   # Check UPnP status
   wraith status --network --verbose
   ```

### Low Throughput

**Problem:** Transfer speed is slow

**Diagnostic:**
```bash
# Check current throughput
wraith status --transfers

# Enable verbose logging
WRAITH_LOG_LEVEL=debug wraith send file.zip --to <peer>
```

**Solutions:**

1. **Reduce obfuscation:**
   ```bash
   wraith send file.zip --to <peer> --obfuscation none
   ```

2. **Check network congestion:**
   ```bash
   # View BBR statistics
   wraith status --network
   ```

3. **Enable kernel bypass (Linux root):**
   ```bash
   sudo wraith daemon --xdp --interface eth0
   ```

### Transfer Failures

**Problem:** Transfer fails mid-way

```bash
# Check error
wraith status --transfers
# Output: Transfer failed: connection lost

# Resume transfer
wraith send file.zip --to <peer> --resume
```

**Solutions:**

1. **Resume interrupted transfer:**
   ```bash
   wraith send file.zip --to <peer> --resume
   ```

2. **Increase timeout:**
   ```toml
   # In config.toml
   [network]
   idle_timeout = "120s"
   ```

3. **Check peer availability:**
   ```bash
   wraith peers
   # Verify peer is online
   ```

### Integrity Verification Failures

**Problem:** Hash mismatch after transfer

```
Error: Integrity verification failed
Expected: a1b2c3d4...
Got:      e5f6g7h8...
```

**Solutions:**

1. **Re-download file:**
   ```bash
   # Delete partial file
   rm ~/Downloads/wraith/file.zip.partial

   # Re-download
   wraith receive --output ~/Downloads/wraith
   ```

2. **Check for network issues:**
   ```bash
   # Enable debug logging
   WRAITH_LOG_LEVEL=debug wraith receive
   ```

3. **Verify source file:**
   ```bash
   # Ask sender to verify their file
   wraith status --transfers
   ```

---

## FAQ

### General

**Q: Is WRAITH anonymous like Tor?**

A: No. WRAITH provides encryption and traffic obfuscation, but peers know each other's IP addresses. For anonymity, use WRAITH over Tor or I2P.

**Q: Can I use WRAITH for legal purposes only?**

A: Yes. WRAITH is designed for legitimate privacy-focused file sharing. Always comply with local laws.

**Q: What operating systems are supported?**

A: Linux (full support), macOS (UDP only, no kernel bypass), Windows (UDP only, experimental).

### Performance

**Q: What throughput can I expect?**

A:
- UDP: 300-500 Mbps (typical)
- AF_XDP: 1-10 Gbps (Linux with compatible NIC)
- Obfuscation reduces throughput proportionally

**Q: Why is my transfer slow?**

A: Common causes:
1. High obfuscation level (try `--obfuscation none`)
2. Network congestion
3. NAT relay fallback (direct connection faster)
4. Peer bandwidth limitations

**Q: How do multi-peer downloads work?**

A: WRAITH divides files into chunks and downloads different chunks from different peers simultaneously, then reassembles them locally.

### Security

**Q: Is WRAITH secure?**

A: Yes, WRAITH uses:
- XChaCha20-Poly1305 for encryption
- Noise_XX handshake with forward secrecy
- BLAKE3 for integrity verification
- Optional traffic obfuscation

**Q: Can my ISP see what I'm transferring?**

A: With obfuscation enabled, your ISP can see encrypted traffic but cannot determine:
- File contents (encrypted)
- File names (encrypted)
- Protocol being used (obfuscated)

Without obfuscation, traffic patterns may reveal you're using WRAITH.

**Q: What happens if my private key is compromised?**

A: Generate a new keypair immediately:
```bash
wraith keygen --force
```
Past sessions with forward secrecy remain protected.

### Network

**Q: Does WRAITH work behind NAT?**

A: Yes. WRAITH supports:
- UDP hole punching
- UPnP/NAT-PMP port mapping
- Relay fallback for strict NAT

**Q: What ports do I need to open?**

A: UDP port 41641 (configurable). TCP is not used.

**Q: Can I run WRAITH as a system service?**

A: Yes, see the deployment guide:
```bash
sudo systemctl enable wraith
sudo systemctl start wraith
```

---

## Security Best Practices

### Key Management

1. **Protect your private key:**
   ```bash
   chmod 600 ~/.config/wraith/keypair.secret
   ```

2. **Back up your keypair:**
   ```bash
   cp ~/.config/wraith/keypair.secret ~/secure-backup/
   ```

3. **Rotate keys periodically:**
   ```bash
   wraith keygen --force  # Generates new identity
   ```

### Network Security

1. **Use obfuscation on untrusted networks:**
   ```bash
   wraith send file.zip --to <peer> --obfuscation high
   ```

2. **Verify peer public keys out-of-band:**
   ```
   Share public keys via secure channel (Signal, in-person, etc.)
   ```

3. **Monitor active connections:**
   ```bash
   wraith status --peers
   ```

### Operational Security

1. **Run daemon with least privilege:**
   ```bash
   # Create dedicated user
   sudo useradd -r -s /bin/false wraith
   sudo -u wraith wraith daemon
   ```

2. **Enable logging for auditing:**
   ```toml
   [logging]
   level = "info"
   file = "/var/log/wraith/wraith.log"
   ```

3. **Keep WRAITH updated:**
   ```bash
   # Check for updates
   wraith --version
   # Update via package manager or rebuild from source
   ```

---

## Getting Help

- **Documentation:** https://github.com/doublegate/WRAITH-Protocol/docs
- **Issues:** https://github.com/doublegate/WRAITH-Protocol/issues
- **Discussions:** https://github.com/doublegate/WRAITH-Protocol/discussions

---

**WRAITH Protocol** - Secure, Private, Fast File Transfer
