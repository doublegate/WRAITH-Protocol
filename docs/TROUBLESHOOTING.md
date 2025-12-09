# WRAITH Protocol Troubleshooting Guide

**Version:** 1.0.0
**Last Updated:** 2025-12-05
**Status:** Complete Troubleshooting Reference

---

## Table of Contents

1. [Connection Issues](#1-connection-issues)
2. [Transfer Issues](#2-transfer-issues)
3. [Discovery Issues](#3-discovery-issues)
4. [Performance Issues](#4-performance-issues)
5. [Common Error Messages](#5-common-error-messages)
6. [Desktop Application Issues](#6-desktop-application-issues-wraith-transfer)

---

## 1. Connection Issues

### 1.1 Cannot Connect to Peers

**Symptoms:**
- Handshake timeout errors
- "Connection refused" messages
- No response from peer
- Session establishment fails

**Diagnostic Commands:**
```bash
# Check if peer is reachable
ping <peer_ip>

# Test UDP connectivity
nc -zvu <peer_ip> 41641

# Check firewall rules
sudo iptables -L -n | grep 41641  # Linux
sudo ufw status                    # Ubuntu
sudo firewall-cmd --list-all       # RHEL/Fedora

# Check WRAITH network status
wraith status --network

# Test STUN connectivity
wraith network test-stun
```

**Common Causes and Solutions:**

1. **Firewall Blocking UDP Port:**
```bash
# Allow UDP port 41641
sudo ufw allow 41641/udp                                  # Ubuntu
sudo firewall-cmd --permanent --add-port=41641/udp        # Fedora
sudo iptables -A INPUT -p udp --dport 41641 -j ACCEPT    # Generic
```

2. **NAT Issues (Symmetric NAT):**
```toml
# Enable relay fallback in config.toml
[discovery]
relay_enabled = true
relay_servers = [
    "relay1.wraith.network:41641",
    "relay2.wraith.network:41641",
]
```

3. **UPnP Not Enabled:**
```toml
[network]
enable_upnp = true
enable_nat_pmp = true
```

4. **Peer Offline or Wrong Address:**
```bash
# Verify peer is running
wraith peers --list

# Try DHT discovery
wraith peers --discover

# Add peer manually if known
wraith peers --add <NODE_ID>@<IP>:<PORT>
```

5. **MTU Issues (Packet Fragmentation):**
```bash
# Test with different MTU sizes
ping -M do -s 1472 <peer_ip>  # Standard MTU
ping -M do -s 1200 <peer_ip>  # Reduced MTU

# If packets are dropped, reduce max_packet_size
```

```toml
[transport]
max_packet_size = 1200  # Reduce from default 1472
```

### 1.2 Frequent Disconnections

**Symptoms:**
- Sessions drop unexpectedly
- High reconnection rate
- Intermittent connectivity
- "Session terminated" in logs

**Diagnostic Commands:**
```bash
# Monitor network stability
mtr <peer_ip>

# Check session logs
sudo journalctl -u wraith | grep "session terminated"

# Monitor active sessions
watch -n 1 'wraith status --network'

# Check for packet loss
wraith status --network --verbose
```

**Common Causes and Solutions:**

1. **Network Instability:**
```toml
[session]
idle_timeout = "120s"  # Increase from 60s
keepalive_interval = "10s"  # More frequent keepalives
max_retransmissions = 10  # More retry attempts
```

2. **Aggressive Timeouts:**
```toml
[session]
handshake_timeout = "30s"  # Increase from 10s
rto_initial = "500ms"  # Increase initial RTO
rto_max = "30s"  # Increase max RTO
```

3. **NAT Session Timeout:**
```toml
[session]
keepalive_interval = "15s"  # Keep NAT mapping alive
```

4. **Connection Migration Failure:**
```bash
# Check PATH_CHALLENGE/PATH_RESPONSE in logs
wraith status --network --debug

# Enable connection migration
```

```toml
[network]
connection_migration_enabled = true
```

### 1.3 Handshake Failures

**Symptoms:**
- "Handshake failed" errors
- Noise_XX handshake timeout
- Authentication failures

**Diagnostic Commands:**
```bash
# Enable debug logging
WRAITH_LOG_LEVEL=debug wraith daemon --foreground

# Check for crypto errors
wraith status --verbose | grep -i handshake

# Verify key integrity
wraith keygen --show-public
```

**Common Causes and Solutions:**

1. **Clock Skew:**
```bash
# Check system time
date
timedatectl status

# Synchronize clocks (both nodes)
sudo ntpdate pool.ntp.org
# Or
sudo timedatectl set-ntp true
```

2. **Corrupt Keypair:**
```bash
# Verify keypair integrity
wraith keygen --verify

# If corrupt, regenerate (WARNING: changes your node ID)
cp ~/.config/wraith/keypair.secret ~/.config/wraith/keypair.secret.backup
wraith keygen --force
```

3. **Network Interference (DPI):**
```bash
# Use high obfuscation
wraith send file.zip --to <peer> --obfuscation paranoid
```

```toml
[obfuscation]
default_level = "high"
tls_mimicry = true
```

4. **Replay Attack Protection:**
```bash
# Check for duplicate nonces in logs
wraith status --debug | grep "replay"

# Verify system entropy
cat /proc/sys/kernel/random/entropy_avail
# Should be > 1000
```

---

## 2. Transfer Issues

### 2.1 Slow Transfer Speeds

**Symptoms:**
- Transfer speed < 10 MB/s on gigabit connection
- High latency between chunks
- Low throughput despite good network

**Diagnostic Commands:**
```bash
# Check current throughput
wraith status --transfers

# Monitor BBR congestion control
wraith status --network --congestion

# Check system resources
top
iotop  # I/O usage
iftop  # Network usage

# Test raw network speed
iperf3 -c <peer_ip>
```

**Common Causes and Solutions:**

1. **High Obfuscation Overhead:**
```bash
# Check current obfuscation level
wraith config show --section obfuscation

# Disable obfuscation for testing
wraith send file.zip --to <peer> --obfuscation none
```

```toml
[obfuscation]
default_level = "none"  # Or "low" for minimal impact
```

2. **Small Chunk Size:**
```toml
[transfer]
chunk_size = 1048576  # Increase to 1 MB from 256 KB
max_parallel_chunks = 32  # More parallelism
```

3. **UDP Buffer Too Small:**
```toml
[transport]
send_buffer_size = 8388608  # 8 MB
recv_buffer_size = 8388608  # 8 MB
```

4. **CPU Bottleneck:**
```bash
# Check CPU usage
top -p $(pidof wraith)

# Enable AF_XDP for kernel bypass (Linux only, requires root)
```

```toml
[transport]
mode = "af-xdp"
interface = "eth0"
```

5. **Disk I/O Bottleneck:**
```bash
# Check I/O wait
iostat -x 1

# Enable io_uring (Linux only)
```

```toml
[files]
backend = "io_uring"
ring_size = 4096
io_polling = true
```

6. **NAT Relay Overhead:**
```bash
# Check if using relay
wraith status --network

# If "Connection Type: Relayed", try to enable direct connection
wraith network test-upnp
```

### 2.2 Transfer Failures Mid-Way

**Symptoms:**
- Transfer stops at random percentage
- "Transfer failed" errors
- Connection lost during transfer

**Diagnostic Commands:**
```bash
# Check transfer status
wraith status --transfers --verbose

# Check error logs
sudo journalctl -u wraith | grep -i "transfer.*failed"

# Verify file integrity
wraith transfer verify <TRANSFER_ID>
```

**Common Causes and Solutions:**

1. **Network Interruption:**
```bash
# Enable automatic resume
wraith send file.zip --to <peer> --resume

# Configuration
```

```toml
[transfer]
resume_enabled = true
resume_ttl = "7d"
```

2. **Disk Space Exhausted:**
```bash
# Check disk space
df -h ~/Downloads

# Clean up old files
rm ~/Downloads/wraith/*.partial

# Increase disk space or change output directory
```

```toml
[transfer]
output_dir = "/mnt/large-disk/wraith"
```

3. **Memory Exhaustion:**
```bash
# Check memory usage
free -h

# Reduce concurrent transfers
```

```toml
[transfer]
max_concurrent_transfers = 3  # Reduce from 10
max_parallel_chunks = 8  # Reduce from 16
```

4. **Integrity Verification Failed:**
```bash
# Check for network corruption
wraith status --network --errors

# Re-download corrupted chunks
wraith transfer retry <TRANSFER_ID>
```

### 2.3 Resume Failures

**Symptoms:**
- Cannot resume interrupted transfer
- "Resume state not found" errors
- Transfer restarts from beginning

**Diagnostic Commands:**
```bash
# List resume states
wraith transfer list-resume

# Check resume directory
ls -lh ~/.local/share/wraith/resume/

# Verify resume state integrity
wraith transfer verify-resume <TRANSFER_ID>
```

**Common Causes and Solutions:**

1. **Resume State Expired:**
```bash
# Check TTL
wraith config show --section transfer | grep resume_ttl

# Increase TTL
```

```toml
[transfer]
resume_ttl = "30d"  # Increase from 7 days
```

2. **Resume State Corruption:**
```bash
# Delete corrupted state and restart
wraith transfer delete-resume <TRANSFER_ID>
wraith send file.zip --to <peer>
```

3. **Peer Changed File:**
```bash
# Verify file hash matches
wraith transfer compare-hash <TRANSFER_ID>

# If hash mismatch, restart transfer completely
wraith transfer delete-resume <TRANSFER_ID>
wraith send file.zip --to <peer>
```

---

## 3. Discovery Issues

### 3.1 DHT Bootstrap Failures

**Symptoms:**
- "Failed to bootstrap DHT" errors
- Cannot discover any peers
- DHT routing table empty

**Diagnostic Commands:**
```bash
# Check DHT status
wraith status --dht

# Test bootstrap node connectivity
nc -zvu bootstrap1.wraith.network 41641

# Check DHT logs
wraith status --debug | grep -i dht
```

**Common Causes and Solutions:**

1. **Bootstrap Nodes Unreachable:**
```bash
# Test each bootstrap node
for node in bootstrap1.wraith.network bootstrap2.wraith.network; do
    echo "Testing $node..."
    nc -zvu $node 41641
done

# Use different bootstrap nodes
```

```toml
[discovery]
bootstrap_nodes = [
    "custom-bootstrap.example.com:41641",
]
```

2. **Firewall Blocking DHT:**
```bash
# Allow DHT traffic (UDP 41641)
sudo ufw allow 41641/udp
```

3. **Clock Skew:**
```bash
# Synchronize time
sudo ntpdate pool.ntp.org
```

4. **DHT Disabled:**
```toml
[discovery]
dht_enabled = true  # Ensure enabled
```

### 3.2 Peer Not Found Errors

**Symptoms:**
- "Peer not found" when trying to connect
- DHT lookup returns no results
- Cannot resolve peer ID to address

**Diagnostic Commands:**
```bash
# Search for peer in DHT
wraith peers --search <NODE_ID>

# List known peers
wraith peers --list

# Check DHT routing table size
wraith status --dht --verbose
```

**Common Causes and Solutions:**

1. **Peer Not Announcing:**
```bash
# Verify peer is announcing to DHT
# On peer node:
wraith status --dht | grep "Announced"

# Enable announcements
```

```toml
[discovery]
dht_enabled = true
announce_interval = "30m"
```

2. **Group Secret Mismatch:**
```bash
# Verify both nodes use same group secret
wraith config show --section discovery | grep group_secret

# Update group secret (both nodes must match)
```

```toml
[discovery]
group_secret = "shared-secret-for-private-group"
```

3. **Peer Recently Joined:**
```bash
# DHT propagation takes time (~5-10 minutes)
# Wait and retry

# Or add peer manually
wraith peers --add <NODE_ID>@<IP>:<PORT>
```

### 3.3 STUN Failures

**Symptoms:**
- "STUN request failed" errors
- Cannot determine public IP
- NAT type detection fails

**Diagnostic Commands:**
```bash
# Test STUN server
wraith network test-stun --server stun.wraith.network:41641

# Check firewall rules
sudo iptables -L -n | grep 41641

# Test with different STUN server
wraith network test-stun --server stun.l.google.com:19302
```

**Common Causes and Solutions:**

1. **STUN Server Unreachable:**
```bash
# Test connectivity
nc -zvu stun.wraith.network 41641

# Use different STUN server
```

```toml
[discovery]
stun_servers = [
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
]
```

2. **Firewall Blocking STUN:**
```bash
# Allow STUN traffic
sudo ufw allow from any to any port 41641 proto udp
```

3. **Symmetric NAT:**
```bash
# Check NAT type
wraith status --network | grep "NAT Type"

# If Symmetric, use relay
```

```toml
[discovery]
relay_enabled = true
```

---

## 4. Performance Issues

### 4.1 High CPU Usage

**Symptoms:**
- CPU usage > 80% during transfers
- System becomes unresponsive
- wraith process using multiple cores

**Diagnostic Commands:**
```bash
# Check CPU usage
top -p $(pidof wraith)
htop

# Profile CPU usage
perf record -p $(pidof wraith) -- sleep 10
perf report

# Check obfuscation overhead
wraith status --network --obfuscation
```

**Common Causes and Solutions:**

1. **High Obfuscation:**
```toml
[obfuscation]
default_level = "low"  # Reduce from high/paranoid
timing_jitter = false
cover_traffic = false
```

2. **Too Many Concurrent Transfers:**
```toml
[transfer]
max_concurrent_transfers = 3  # Reduce from 10
max_parallel_chunks = 8  # Reduce from 16
```

3. **Crypto Overhead:**
```bash
# Check if CPU supports AES-NI
grep aes /proc/cpuinfo

# Verify SIMD is enabled
wraith --version | grep -i simd
```

4. **Enable Kernel Bypass:**
```toml
[transport]
mode = "af-xdp"  # Offload to kernel
interface = "eth0"
```

### 4.2 High Memory Usage

**Symptoms:**
- Memory usage > 2 GB
- Out of memory errors
- System swapping heavily

**Diagnostic Commands:**
```bash
# Check memory usage
free -h
ps aux | grep wraith | awk '{print $6}'

# Check for memory leaks
valgrind --leak-check=full wraith daemon --foreground

# Monitor memory over time
watch -n 1 'ps aux | grep wraith'
```

**Common Causes and Solutions:**

1. **Too Many Sessions:**
```toml
[session]
max_sessions = 100  # Reduce from 1000
```

2. **Large UMEM (AF_XDP):**
```toml
[transport]
xdp_umem_size = 4194304  # 4 MB instead of 16 MB
```

3. **Large Buffer Sizes:**
```toml
[transport]
send_buffer_size = 1048576  # 1 MB instead of 2 MB
recv_buffer_size = 1048576
```

4. **Too Many Concurrent Chunks:**
```toml
[transfer]
max_parallel_chunks = 8  # Reduce from 16
```

### 4.3 Disk I/O Bottleneck

**Symptoms:**
- High I/O wait percentage
- Slow file operations
- Transfer speed limited by disk

**Diagnostic Commands:**
```bash
# Check I/O wait
iostat -x 1

# Monitor disk usage
iotop

# Check disk performance
sudo hdparm -tT /dev/sda

# Monitor file operations
watch -n 1 'wraith status --transfers --verbose'
```

**Common Causes and Solutions:**

1. **Enable io_uring:**
```toml
[files]
backend = "io_uring"
ring_size = 4096
io_polling = true
```

2. **Enable Direct I/O:**
```toml
[files]
direct_io = true  # Bypass page cache
preallocate = true
```

3. **Increase Buffer Sizes:**
```toml
[files]
read_buffer_size = 4194304  # 4 MB
write_buffer_size = 4194304
```

4. **Use Faster Storage:**
```bash
# Move to SSD
mv ~/Downloads /mnt/ssd/Downloads
```

```toml
[transfer]
output_dir = "/mnt/ssd/wraith"
```

---

## 5. Common Error Messages

### 5.1 "Session not found"

**Meaning:** Attempted operation on non-existent or closed session.

**Solutions:**
```bash
# Establish session first
wraith peers --connect <NODE_ID>

# Or use get_or_establish API
```

**Code Fix:**
```rust
// Use get_or_establish instead of get_session
let session_id = node.get_or_establish_session(peer_id).await?;
```

### 5.2 "Transport initialization failed"

**Meaning:** Could not initialize network transport (AF_XDP or UDP).

**Solutions:**
```bash
# Check interface name
ip link show

# Verify AF_XDP support
wraith network test-afxdp --interface eth0

# Fall back to UDP
```

```toml
[transport]
mode = "udp"  # Force UDP instead of auto/af-xdp
```

### 5.3 "Handshake timeout"

**Meaning:** Noise_XX handshake did not complete within timeout.

**Solutions:**
```bash
# Increase timeout
```

```toml
[session]
handshake_timeout = "30s"
```

```bash
# Check network latency
ping <peer_ip>

# Check firewall
sudo iptables -L -n | grep 41641
```

### 5.4 "BLAKE3 hash mismatch"

**Meaning:** File integrity verification failed (corruption detected).

**Solutions:**
```bash
# Delete partial file and restart
rm ~/Downloads/wraith/<file>.partial
wraith receive --output ~/Downloads

# Check network for corruption
mtr <peer_ip>

# Re-download from different peer
wraith transfer cancel <TRANSFER_ID>
wraith receive --multi-peer
```

### 5.5 "Rate limit exceeded"

**Meaning:** Connection or packet rate limit triggered.

**Solutions:**
```bash
# Wait and retry
sleep 60
wraith send file.zip --to <peer>

# Adjust rate limits
```

```toml
[network]
connection_rate_limit = 200  # Increase from 100
```

### 5.6 "Replay attack detected"

**Meaning:** Duplicate nonce detected (possible replay attack).

**Solutions:**
```bash
# Check system time
date
sudo ntpdate pool.ntp.org

# Verify entropy
cat /proc/sys/kernel/random/entropy_avail

# If legitimate, increase replay window
```

```toml
[security]
replay_window_size = 2048  # Increase from 1024
```

### 5.7 "NAT traversal failed"

**Meaning:** Could not establish direct connection or relay.

**Solutions:**
```bash
# Check NAT type
wraith status --network | grep "NAT Type"

# Enable relay fallback
```

```toml
[discovery]
relay_enabled = true
relay_servers = [
    "relay1.wraith.network:41641",
    "relay2.wraith.network:41641",
]
```

### 5.8 "Circuit breaker open"

**Meaning:** Too many consecutive failures, circuit breaker activated.

**Solutions:**
```bash
# Wait for recovery timeout
sleep 30

# Or reset circuit breaker
wraith network reset-circuit-breaker <PEER_ID>

# Check underlying cause
wraith status --network --debug
```

### 5.9 "Health monitor: CRITICAL state"

**Meaning:** Node under resource pressure (>90% memory).

**Solutions:**
```bash
# Check memory usage
free -h

# Stop non-essential transfers
wraith transfer cancel <TRANSFER_ID>

# Restart daemon
sudo systemctl restart wraith

# Increase system resources
```

### 5.10 "Peer blocked"

**Meaning:** Peer is in blocked list.

**Solutions:**
```bash
# Verify blocked list
wraith peers --list-blocked

# Unblock peer
wraith peers --unblock <NODE_ID>
```

---

## 6. Desktop Application Issues (WRAITH Transfer)

### 6.1 Wayland Display Protocol Warnings

**Symptoms:**
```
Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.
```

**Cause:**
- Tauri GUI framework initializes display subsystems even for non-GUI operations
- GTK/GDK emits protocol warnings during cleanup on Wayland
- This is a harmless warning from the underlying Tauri framework

**Impact:**
- None - the application functions correctly despite this warning
- Warning appears during initialization/cleanup, not during normal operation
- Does not affect file transfer functionality or GUI behavior

**Solutions:**

1. **Ignore the Warning (Recommended)**
   - The warning is harmless and can be safely ignored
   - Application functionality is not affected

2. **Suppress stderr Output (For Scripts)**
   ```bash
   # Suppress all stderr warnings
   wraith-transfer 2>/dev/null

   # Capture only stdout, discard stderr
   wraith-transfer > output.txt 2>/dev/null
   ```

3. **Known Issue**
   - This is a Tauri framework limitation
   - Reported upstream to Tauri project
   - No application-level fix available without modifying Tauri internals

**Note:** Previous versions also showed a `libayatana-appindicator` warning, which has been fixed by removing unused system tray functionality in v0.1.0+.

### 6.2 Application Won't Start

**Symptoms:**
- Window doesn't appear
- Application crashes on startup
- "Failed to initialize GTK backend" errors

**Diagnostic Commands:**
```bash
# Check GTK installation
pkg-config --modversion gtk+-3.0

# Verify Wayland/X11 display
echo $WAYLAND_DISPLAY
echo $DISPLAY

# Test with debug output
RUST_BACKTRACE=1 wraith-transfer
```

**Common Causes and Solutions:**

1. **Missing GTK Dependencies**
   ```bash
   # Ubuntu/Debian
   sudo apt install libgtk-3-0 libwebkit2gtk-4.1-0

   # Fedora/RHEL
   sudo dnf install gtk3 webkit2gtk4.1

   # Arch Linux
   sudo pacman -S gtk3 webkit2gtk
   ```

2. **Display Server Issues**
   ```bash
   # Check if display is available
   if [ -z "$DISPLAY" ] && [ -z "$WAYLAND_DISPLAY" ]; then
       echo "No display server available"
   fi

   # Try running with X11 backend (if Wayland fails)
   GDK_BACKEND=x11 wraith-transfer
   ```

3. **Permission Issues**
   ```bash
   # Check if binary is executable
   chmod +x /path/to/wraith-transfer

   # Verify no AppArmor/SELinux restrictions
   sudo aa-status  # AppArmor
   sudo sestatus   # SELinux
   ```

### 6.3 File Picker Not Working

**Symptoms:**
- File selection dialog doesn't appear
- Dialog appears but files can't be selected
- Permission denied errors

**Solutions:**

1. **Portal Permissions (Flatpak/Snap)**
   ```bash
   # Grant file access permissions
   flatpak permission-set desktop-used-apps wraith-transfer yes

   # Or use XDG portal
   xdg-desktop-portal --version
   ```

2. **Check Sandbox Restrictions**
   ```bash
   # Disable sandbox for testing (development only)
   WEBKIT_DISABLE_COMPOSITING_MODE=1 wraith-transfer
   ```

### 6.4 High Memory/CPU Usage

**Symptoms:**
- Application uses excessive RAM (>500 MB)
- CPU usage high when idle
- System becomes sluggish

**Diagnostic Commands:**
```bash
# Monitor resource usage
top -p $(pgrep wraith-transfer)

# Check WebView memory
ps aux | grep -i webkit

# Monitor heap allocations (requires valgrind)
valgrind --tool=massif wraith-transfer
```

**Solutions:**

1. **Close Unused Transfers**
   - Active transfers consume memory for chunk buffers
   - Close completed or failed transfers to free resources

2. **Reduce Concurrent Transfers**
   - Settings → Transfer → Max Concurrent Transfers
   - Lower value reduces memory usage

3. **Disable Logging (If Excessive)**
   - Settings → Logging → Set to "Warn" or "Error"
   - Verbose logging increases CPU usage

---

## Getting More Help

If you've tried the solutions in this guide and still experience issues:

1. **Enable Debug Logging:**
```bash
WRAITH_LOG_LEVEL=debug wraith daemon --foreground > debug.log 2>&1
```

2. **Collect Diagnostic Information:**
```bash
wraith status --verbose > status.txt
wraith config show > config.txt
journalctl -u wraith > logs.txt
```

3. **Check GitHub Issues:**
   - [Issue Tracker](https://github.com/doublegate/WRAITH-Protocol/issues)
   - Search for similar problems

4. **Ask in Discussions:**
   - [GitHub Discussions](https://github.com/doublegate/WRAITH-Protocol/discussions)
   - Community support

5. **File a Bug Report:**
   - Include debug logs
   - Describe steps to reproduce
   - System information (OS, version, hardware)

---

**WRAITH Protocol Troubleshooting Guide**

**Version:** 1.0.0 | **License:** MIT | **Language:** Rust 2024
