# WRAITH Protocol Troubleshooting

**Document Version:** 1.0.0
**Last Updated:** 2025-11-28
**Status:** Operations Documentation

---

## Overview

This guide provides solutions to common issues encountered when deploying and operating WRAITH Protocol nodes.

**Troubleshooting Approach:**
1. Identify symptoms
2. Collect diagnostic information
3. Isolate the problem
4. Apply solution
5. Verify fix

---

## Common Issues

### Connection Problems

#### Issue: Cannot Connect to Peer

**Symptoms:**
- Handshake timeout errors
- "Connection refused" messages
- No response from peer

**Diagnosis:**
```bash
# Check if peer is reachable
ping <peer_ip>

# Check if port is open
nc -zvu <peer_ip> 41641

# Test UDP connectivity
wraith-cli ping --peer <peer_ip>:41641

# Check firewall rules
sudo iptables -L -n | grep 41641
```

**Solutions:**

1. **Firewall blocking:**
```bash
# Allow UDP port 41641
sudo ufw allow 41641/udp
sudo ufw reload
```

2. **NAT issues:**
```bash
# Enable UPnP in config
[network]
enable_upnp = true
enable_nat_pmp = true
```

3. **Peer offline:**
```bash
# Verify peer is running
ssh peer-host "systemctl status wraith"
```

#### Issue: Frequent Disconnections

**Symptoms:**
- Sessions drop unexpectedly
- High reconnection rate
- Intermittent connectivity

**Diagnosis:**
```bash
# Check session logs
sudo journalctl -u wraith | grep "session terminated"

# Monitor network stability
mtr <peer_ip>

# Check MTU issues
ping -M do -s 1472 <peer_ip>
```

**Solutions:**

1. **MTU misconfiguration:**
```toml
[transport]
max_packet_size = 1200  # Reduce from 1472
```

2. **Timeout too aggressive:**
```toml
[session]
idle_timeout = "60s"  # Increase from 30s
```

3. **Network congestion:**
```bash
# Enable BBR congestion control
sudo sysctl -w net.ipv4.tcp_congestion_control=bbr
```

---

### Performance Issues

#### Issue: Low Throughput

**Symptoms:**
- Transfer speed much slower than expected
- High CPU usage but low network utilization
- Packet loss

**Diagnosis:**
```bash
# Check current throughput
wraith-cli stats

# Network interface stats
ip -s link show eth0

# CPU usage per core
mpstat -P ALL 1

# Check for packet loss
netstat -s | grep -i loss
```

**Solutions:**

1. **Buffer sizes too small:**
```bash
# Increase socket buffers
sudo sysctl -w net.core.rmem_max=26214400
sudo sysctl -w net.core.wmem_max=26214400
```

2. **Not using AF_XDP:**
```toml
[transport]
mode = "af-xdp"  # Instead of "udp"
interface = "eth0"
```

3. **Chunk size suboptimal:**
```toml
[files]
chunk_size = 1048576  # 1 MB chunks
max_parallel_chunks = 16
```

4. **CPU pinning:**
```bash
# Pin to specific cores
taskset -c 0-3 wraith-cli daemon
```

#### Issue: High Latency

**Symptoms:**
- Slow handshake completion
- DHT lookups take >1 second
- File transfers delayed

**Diagnosis:**
```bash
# Measure RTT
ping -c 10 <peer_ip>

# Check DHT latency
wraith-cli dht lookup <key> --verbose

# Network path analysis
traceroute <peer_ip>
```

**Solutions:**

1. **Geographic distance:**
```toml
# Use geographically closer relay
[relay]
servers = ["relay-us-west.wraith.network:41641"]
```

2. **Query timeout:**
```toml
[dht]
query_timeout = "5s"  # Increase from 2s
```

---

### Cryptographic Issues

#### Issue: Handshake Failures

**Symptoms:**
- "Decryption failed" errors
- "Invalid signature" messages
- Authentication failures

**Diagnosis:**
```bash
# Check keypair validity
wraith-cli keypair verify --keypair /etc/wraith/keypair.secret

# Test handshake with verbose logging
RUST_LOG=debug wraith-cli connect <peer>
```

**Solutions:**

1. **Corrupted keypair:**
```bash
# Regenerate keypair
mv /etc/wraith/keypair.secret /etc/wraith/keypair.secret.bak
wraith-cli keygen --output /etc/wraith/keypair.secret
```

2. **Clock skew:**
```bash
# Sync system clock
sudo ntpdate pool.ntp.org
# Or use systemd-timesyncd
sudo timedatectl set-ntp true
```

3. **Incompatible protocol version:**
```bash
# Check versions
wraith-cli --version  # Both peers
```

#### Issue: Replay Attack Detected

**Symptoms:**
- "Replay attack detected" errors
- Nonce errors in logs

**Diagnosis:**
```bash
# Check for duplicate packets
sudo tcpdump -i eth0 'udp port 41641' -vv | grep -i duplicate
```

**Solutions:**

1. **Network issue (packet duplication):**
```bash
# Check network interface for errors
ethtool -S eth0 | grep -i error
```

2. **System compromise:**
```bash
# Rotate keys immediately
wraith-cli keygen --output /etc/wraith/keypair.secret.new
# Update configuration
# Restart service
```

---

### DHT Issues

#### Issue: Cannot Find Peers

**Symptoms:**
- DHT lookups return empty
- "No peers found" errors
- File transfers cannot start

**Diagnosis:**
```bash
# Check DHT status
wraith-cli dht status

# Test bootstrap nodes
for node in dht1 dht2 dht3; do
    nc -zvu $node.wraith.network 41641
done

# Check DHT peer count
wraith-cli dht peers | wc -l
```

**Solutions:**

1. **Bootstrap node unreachable:**
```toml
# Use alternative bootstrap nodes
[dht]
bootstrap_nodes = [
    "backup-dht1.wraith.network:41641",
    "backup-dht2.wraith.network:41641",
]
```

2. **Firewall blocking DHT:**
```bash
# Allow DHT traffic (same port as main protocol)
sudo ufw allow 41641/udp
```

3. **Isolated network:**
```bash
# Manual peer addition
wraith-cli peer add <peer_ip>:41641
```

---

### Resource Issues

#### Issue: Out of Memory

**Symptoms:**
- Process killed by OOM killer
- "Cannot allocate memory" errors
- System freezes

**Diagnosis:**
```bash
# Check memory usage
free -h
ps aux | grep wraith

# Check OOM killer logs
sudo journalctl -k | grep -i "killed process"

# Monitor memory in real-time
watch -n 1 'ps -p $(pgrep wraith-cli) -o %mem,rss,vsz'
```

**Solutions:**

1. **Set memory limits:**
```bash
# Systemd service limit
[Service]
MemoryMax=2G
MemoryHigh=1.5G
```

2. **Reduce session limit:**
```toml
[session]
max_concurrent_sessions = 100  # Reduce from 1000
```

3. **Increase swap:**
```bash
# Add 4 GB swap
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

#### Issue: File Descriptor Exhaustion

**Symptoms:**
- "Too many open files" errors
- Cannot accept new connections
- Service degradation

**Diagnosis:**
```bash
# Check current usage
lsof -p $(pgrep wraith-cli) | wc -l

# Check limit
ulimit -n
```

**Solutions:**

1. **Increase system limit:**
```bash
# /etc/security/limits.conf
wraith soft nofile 65536
wraith hard nofile 65536
```

2. **Systemd service limit:**
```bash
[Service]
LimitNOFILE=65536
```

---

### AF_XDP Issues

#### Issue: Permission Denied (XDP)

**Symptoms:**
- "Operation not permitted" when starting
- Cannot bind AF_XDP socket
- XDP program load fails

**Diagnosis:**
```bash
# Check capabilities
getcap /usr/local/bin/wraith-cli

# Check kernel support
zgrep CONFIG_XDP /proc/config.gz

# Test with sudo (not recommended for prod)
sudo wraith-cli daemon --transport af-xdp
```

**Solutions:**

1. **Grant capabilities:**
```bash
sudo setcap cap_net_raw,cap_net_admin,cap_bpf+ep /usr/local/bin/wraith-cli
```

2. **Run as privileged user:**
```bash
# Systemd service
[Service]
User=root  # Not recommended
Group=root
```

#### Issue: XDP Not Supported

**Symptoms:**
- "XDP not supported on this interface" error
- Driver doesn't support XDP

**Diagnosis:**
```bash
# Check driver
ethtool -i eth0 | grep driver

# Test XDP support
sudo ip link set dev eth0 xdp obj simple_xdp.o sec xdp
```

**Solutions:**

1. **Use compatible NIC:**
```
Supported drivers: i40e, ixgbe, mlx5, virtio_net (kernel 6.2+)
```

2. **Fallback to UDP:**
```toml
[transport]
mode = "udp"  # Instead of af-xdp
```

---

## Diagnostic Commands

### General Health Check

```bash
#!/bin/bash
# wraith-diag.sh - Comprehensive diagnostic script

echo "=== WRAITH Diagnostic Report ==="
echo

echo "1. Service Status"
systemctl status wraith
echo

echo "2. Process Info"
ps aux | grep wraith
echo

echo "3. Network Connectivity"
ss -lun | grep 41641
echo

echo "4. Memory Usage"
ps -p $(pgrep wraith-cli) -o %mem,rss,vsz
echo

echo "5. Recent Errors"
sudo journalctl -u wraith --since "1 hour ago" | grep -i error
echo

echo "6. DHT Status"
wraith-cli dht status
echo

echo "7. Active Sessions"
wraith-cli sessions list
echo

echo "=== End Report ==="
```

### Log Analysis

```bash
# Extract error patterns
sudo journalctl -u wraith --since today | \
    grep -i error | \
    awk '{print $NF}' | \
    sort | uniq -c | sort -rn

# Find slow operations
sudo journalctl -u wraith --since "1 hour ago" | \
    grep "duration_ms" | \
    awk '{print $NF}' | \
    sort -n | tail -20

# Count event types
sudo journalctl -u wraith --since today -o json | \
    jq -r '.MESSAGE' | \
    grep "Session\|Transfer\|DHT" | \
    sort | uniq -c
```

---

## Recovery Procedures

### Service Recovery

```bash
# 1. Stop service
sudo systemctl stop wraith

# 2. Backup current state
sudo tar czf /tmp/wraith-backup-$(date +%Y%m%d%H%M).tar.gz \
    /etc/wraith \
    /var/lib/wraith

# 3. Clear state (if needed)
sudo rm -rf /var/lib/wraith/sessions/*
sudo rm -rf /var/lib/wraith/dht/*

# 4. Restart service
sudo systemctl start wraith

# 5. Verify
sudo systemctl status wraith
wraith-cli status
```

### Configuration Reset

```bash
# Backup current config
sudo cp /etc/wraith/config.toml /etc/wraith/config.toml.bak

# Generate default config
wraith-cli config init > /tmp/default-config.toml

# Merge configurations
sudo cp /tmp/default-config.toml /etc/wraith/config.toml

# Restart
sudo systemctl restart wraith
```

---

## Getting Help

### Collecting Debug Information

```bash
# Generate debug bundle
wraith-cli debug-bundle --output /tmp/wraith-debug.tar.gz

# Contents:
# - Configuration (sanitized)
# - Logs (last 24h)
# - System info
# - Network diagnostics
# - Performance metrics
```

### Community Support

- **GitHub Issues:** https://github.com/doublegate/WRAITH-Protocol/issues
- **Discord:** https://discord.gg/wraith (if available)
- **Email:** support@wraith.network

### Reporting Bugs

**Include:**
1. WRAITH version (`wraith-cli --version`)
2. Operating system and kernel version
3. Error messages and logs
4. Steps to reproduce
5. Expected vs. actual behavior
6. Debug bundle (if possible)

---

## See Also

- [Deployment Guide](deployment-guide.md)
- [Monitoring](monitoring.md)
- [Development Guide](../engineering/development-guide.md)
