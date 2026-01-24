# WRAITH Protocol Deployment Runbook

**Version:** 1.0
**Last Updated:** 2025-12-01
**Maintainer:** WRAITH Operations Team

---

## Overview

This runbook provides step-by-step instructions for deploying WRAITH Protocol nodes in production environments. Follow these procedures to ensure secure, reliable deployments.

---

## Prerequisites

### System Requirements

**Hardware:**
- CPU: 2+ cores (4+ recommended)
- RAM: 4 GB minimum (8 GB recommended)
- Storage: 50 GB minimum for node operation
- Network: 1 Gbps network interface (10 Gbps for high-throughput)

**Operating System:**
- Linux: Ubuntu 22.04 LTS, Debian 12, RHEL 9, or compatible
- Kernel: 6.2+ (for AF_XDP and io_uring support)
- Architecture: x86_64 or aarch64

**Software:**
- Rust: 1.88+ (MSRV)
- Build tools: gcc, make, pkg-config
- Libraries: libpcap-dev (optional for packet capture)

### Pre-Deployment Checklist

- [ ] System meets hardware requirements
- [ ] Operating system updated to latest patches
- [ ] Firewall rules configured (see Network Configuration)
- [ ] SSL/TLS certificates obtained (if using relay)
- [ ] SSH keys configured for remote access
- [ ] Monitoring infrastructure ready
- [ ] Backup system configured
- [ ] Disaster recovery plan documented

---

## Installation

### 1. Build from Source

```bash
# Clone repository
git clone https://github.com/doublegate/WRAITH-Protocol.git
cd WRAITH-Protocol

# Checkout stable version
git checkout v0.8.0

# Build release binary
cargo build --release --workspace

# Verify build
./target/release/wraith --version
```

### 2. System Configuration

```bash
# Create wraith user (no login shell)
sudo useradd -r -s /bin/false -m -d /var/lib/wraith wraith

# Create necessary directories
sudo mkdir -p /etc/wraith
sudo mkdir -p /var/lib/wraith/{keys,data,logs}
sudo mkdir -p /var/run/wraith

# Set ownership
sudo chown -R wraith:wraith /var/lib/wraith
sudo chown -R wraith:wraith /var/run/wraith
sudo chmod 700 /var/lib/wraith/keys
```

### 3. Install Binary

```bash
# Install to system location
sudo install -m 755 target/release/wraith /usr/local/bin/

# Verify installation
wraith --version
```

### 4. Generate Node Keys

```bash
# Generate Ed25519 keypair (identity)
sudo -u wraith wraith keygen -o /var/lib/wraith/keys/node_key.enc

# Set restrictive permissions
sudo chmod 600 /var/lib/wraith/keys/node_key.enc

# Backup keypair (encrypted)
sudo cp /var/lib/wraith/keys/node_key.enc /var/lib/wraith/keys/node_key.enc.backup
```

---

## Configuration

### 1. Create Configuration File

```bash
# Generate default config
sudo -u wraith wraith config --generate > /tmp/wraith.toml

# Move to system location
sudo mv /tmp/wraith.toml /etc/wraith/config.toml
sudo chown wraith:wraith /etc/wraith/config.toml
sudo chmod 640 /etc/wraith/config.toml
```

### 2. Edit Configuration

Edit `/etc/wraith/config.toml`:

```toml
[node]
listen_addr = "0.0.0.0:5000"
key_file = "/var/lib/wraith/keys/node_key.enc"
data_dir = "/var/lib/wraith/data"

[network]
max_connections = 1000
connection_timeout = 30
bandwidth_limit = 1000000000  # 1 Gbps

[obfuscation]
mode = "tls"  # Options: none, tls, websocket, doh
padding_mode = "statistical"
timing_mode = "uniform"

[discovery]
dht_enabled = true
bootstrap_nodes = [
    "bootstrap1.wraith.example:5000",
    "bootstrap2.wraith.example:5000",
]
relay_enabled = true

[transfer]
chunk_size = 262144  # 256 KiB
max_concurrent_transfers = 100

[logging]
level = "info"
file = "/var/lib/wraith/logs/wraith.log"
max_size = "100MB"
max_age = "30d"
```

### 3. Network Configuration

#### Firewall Rules (iptables)

```bash
# Allow WRAITH protocol (UDP + TCP)
sudo iptables -A INPUT -p udp --dport 5000 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 5000 -j ACCEPT

# Allow outbound connections
sudo iptables -A OUTPUT -p udp --dport 5000 -j ACCEPT
sudo iptables -A OUTPUT -p tcp --dport 5000 -j ACCEPT

# Save rules
sudo iptables-save > /etc/iptables/rules.v4
```

#### Firewall Rules (firewalld)

```bash
# Add service
sudo firewall-cmd --permanent --new-service=wraith
sudo firewall-cmd --permanent --service=wraith --add-port=5000/udp
sudo firewall-cmd --permanent --service=wraith --add-port=5000/tcp
sudo firewall-cmd --permanent --add-service=wraith
sudo firewall-cmd --reload
```

#### NAT Configuration (if behind NAT)

```bash
# Port forwarding (external 5000 -> internal 5000)
sudo iptables -t nat -A PREROUTING -p udp --dport 5000 -j DNAT --to-destination 192.168.1.100:5000
sudo iptables -t nat -A PREROUTING -p tcp --dport 5000 -j DNAT --to-destination 192.168.1.100:5000
```

---

## Service Management

### 1. Create systemd Service

Create `/etc/systemd/system/wraith.service`:

```ini
[Unit]
Description=WRAITH Protocol Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=wraith
Group=wraith
ExecStart=/usr/local/bin/wraith daemon --config /etc/wraith/config.toml
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=wraith

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/wraith /var/run/wraith
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
RestrictNamespaces=true
LockPersonality=true
MemoryDenyWriteExecute=true
RestrictRealtime=true
RestrictSUIDSGID=true
PrivateMounts=true

# Resource limits
LimitNOFILE=65536
LimitNPROC=512

[Install]
WantedBy=multi-user.target
```

### 2. Enable and Start Service

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

---

## Verification

### 1. Check Node Status

```bash
# Check if node is running
sudo systemctl status wraith

# Check process
ps aux | grep wraith

# Check listening ports
sudo ss -tulnp | grep 5000

# Check node health
wraith status
```

### 2. Verify Connectivity

```bash
# Check DHT connectivity
wraith peers

# Test file transfer (to another node)
wraith send <file> <node_id>

# Monitor transfers
wraith transfers
```

### 3. Check Logs

```bash
# Follow logs in real-time
sudo journalctl -u wraith -f

# Check for errors
sudo journalctl -u wraith -p err

# Check recent logs
sudo journalctl -u wraith --since "1 hour ago"
```

---

## Monitoring

### 1. Health Checks

```bash
# HTTP health endpoint (if enabled)
curl http://localhost:8080/health

# Expected response:
# {"status": "healthy", "uptime": 3600, "connections": 42}
```

### 2. Metrics Collection

**Prometheus Integration:**

Add to `/etc/wraith/config.toml`:

```toml
[metrics]
enabled = true
listen_addr = "127.0.0.1:9090"
path = "/metrics"
```

Prometheus scrape config:

```yaml
scrape_configs:
  - job_name: 'wraith'
    static_configs:
      - targets: ['localhost:9090']
```

### 3. Log Monitoring

```bash
# Set up log rotation
sudo tee /etc/logrotate.d/wraith <<EOF
/var/lib/wraith/logs/*.log {
    daily
    rotate 30
    compress
    delaycompress
    notifempty
    missingok
    copytruncate
}
EOF
```

---

## Backup & Recovery

### 1. Backup Node Keys

```bash
# Backup encrypted keys
sudo cp /var/lib/wraith/keys/node_key.enc /backup/wraith/node_key.enc.$(date +%Y%m%d)

# Backup configuration
sudo cp /etc/wraith/config.toml /backup/wraith/config.toml.$(date +%Y%m%d)

# Store backup securely (off-site)
```

### 2. Restore from Backup

```bash
# Stop service
sudo systemctl stop wraith

# Restore keys
sudo cp /backup/wraith/node_key.enc.20251201 /var/lib/wraith/keys/node_key.enc
sudo chown wraith:wraith /var/lib/wraith/keys/node_key.enc
sudo chmod 600 /var/lib/wraith/keys/node_key.enc

# Restore config
sudo cp /backup/wraith/config.toml.20251201 /etc/wraith/config.toml
sudo chown wraith:wraith /etc/wraith/config.toml

# Start service
sudo systemctl start wraith
```

---

## Scaling

### Horizontal Scaling (Multiple Nodes)

1. Deploy multiple nodes with unique keys
2. Configure load balancer (HAProxy, nginx)
3. Use shared DHT bootstrap nodes
4. Monitor aggregate throughput

### Vertical Scaling (Single Node)

1. Increase CPU cores (4 -> 8 -> 16)
2. Increase RAM (8 GB -> 16 GB -> 32 GB)
3. Use faster storage (SSD -> NVMe)
4. Enable AF_XDP for kernel bypass
5. Tune `max_connections` and `bandwidth_limit`

---

## Troubleshooting

**Node won't start:**
- Check logs: `sudo journalctl -u wraith -n 100`
- Verify config syntax: `wraith config --validate`
- Check port availability: `sudo ss -tulnp | grep 5000`

**High CPU usage:**
- Check active transfers: `wraith transfers`
- Review obfuscation mode (statistical padding is CPU-intensive)
- Enable BLAKE3 SIMD acceleration

**High memory usage:**
- Reduce `max_concurrent_transfers`
- Check for memory leaks in logs
- Review chunk size (larger = more memory)

**Slow transfer speeds:**
- Check network bandwidth: `iftop`, `nethogs`
- Review BBR congestion control settings
- Verify SIMD features enabled (cargo build --features simd-avx2)
- Check relay vs direct connection

---

## Security Hardening

### System Hardening

```bash
# Disable unnecessary services
sudo systemctl disable bluetooth
sudo systemctl disable cups

# Enable automatic security updates
sudo apt install unattended-upgrades
sudo dpkg-reconfigure -plow unattended-upgrades

# Configure fail2ban (optional)
sudo apt install fail2ban
```

### WRAITH Hardening

1. Use strong encryption for node keys (Argon2id)
2. Enable TLS mimicry for obfuscation
3. Disable relay if not needed
4. Use allow-list for peers (if applicable)
5. Enable rate limiting

---

## Rollback Procedure

```bash
# Stop current version
sudo systemctl stop wraith

# Backup current binary
sudo cp /usr/local/bin/wraith /usr/local/bin/wraith.backup

# Install previous version
sudo install -m 755 /backup/wraith-v0.7.0 /usr/local/bin/wraith

# Restore previous config (if needed)
sudo cp /backup/wraith/config.toml.v0.7.0 /etc/wraith/config.toml

# Start service
sudo systemctl start wraith

# Verify rollback
wraith --version
sudo systemctl status wraith
```

---

## Support Contacts

- **Technical Support:** support@wraith.example
- **Security Issues:** security@wraith.example
- **On-Call:** +1-555-WRAITH-OPS

---

## Appendix

### A. Environment Variables

- `WRAITH_CONFIG`: Override config file path
- `WRAITH_LOG_LEVEL`: Override log level (debug, info, warn, error)
- `RUST_BACKTRACE`: Enable backtraces (1 or full)

### B. Performance Tuning

```bash
# Increase file descriptor limit
sudo sysctl -w fs.file-max=100000

# Tune network buffers
sudo sysctl -w net.core.rmem_max=134217728
sudo sysctl -w net.core.wmem_max=134217728
sudo sysctl -w net.ipv4.tcp_rmem="4096 87380 134217728"
sudo sysctl -w net.ipv4.tcp_wmem="4096 65536 134217728"

# Enable BBR congestion control
sudo sysctl -w net.ipv4.tcp_congestion_control=bbr
sudo sysctl -w net.core.default_qdisc=fq
```

### C. Useful Commands

```bash
# Check node version
wraith --version

# Validate config
wraith config --validate /etc/wraith/config.toml

# Generate new keypair
wraith keygen -o /path/to/key.enc

# List peers
wraith peers

# Show active transfers
wraith transfers

# Node statistics
wraith stats
```
