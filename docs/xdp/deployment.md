# XDP Deployment Guide

## Overview

This guide covers deploying WRAITH with XDP acceleration in production environments, including system configuration, monitoring, and troubleshooting.

## Pre-Deployment Checklist

### System Requirements

- [ ] Linux kernel 6.2+ (check: `uname -r`)
- [ ] Compatible NIC driver (check: `ethtool -i eth0 | grep driver`)
- [ ] Multi-queue NIC (check: `ethtool -l eth0`)
- [ ] Locked memory limit set (check: `ulimit -l` = unlimited)
- [ ] Capabilities granted or running as root
- [ ] Build dependencies installed (Rust 1.88+)

### Network Requirements

- [ ] Static IP address assigned to NIC
- [ ] Firewall rules configured (if applicable)
- [ ] MTU size verified (check: `ip link show eth0 | grep mtu`)
- [ ] Network switch supports jumbo frames (optional, for 10G+)

### Security Requirements

- [ ] Private keys generated and encrypted
- [ ] Configuration file permissions restricted (0600)
- [ ] Audit logging enabled
- [ ] SELinux/AppArmor policies configured (if applicable)

## Step-by-Step Deployment

### Step 1: Build WRAITH

```bash
# Clone repository
git clone https://github.com/doublegate/WRAITH-Protocol.git
cd WRAITH-Protocol

# Build release binary
cargo build --release --bin wraith

# Install to /usr/local/bin
sudo install -m 755 target/release/wraith /usr/local/bin/
```

### Step 2: Grant Capabilities

```bash
# Grant required capabilities (instead of running as root)
sudo setcap \
  cap_net_raw=ep,cap_net_admin=ep,cap_bpf=ep,cap_ipc_lock=ep \
  /usr/local/bin/wraith

# Verify
getcap /usr/local/bin/wraith
```

### Step 3: Configure System

#### Increase Locked Memory Limit

```bash
# Edit /etc/security/limits.conf
sudo tee -a /etc/security/limits.conf <<EOF
*    soft    memlock    unlimited
*    hard    memlock    unlimited
EOF

# Apply immediately (logout/login to persist)
ulimit -l unlimited
```

#### Configure Huge Pages (Optional)

```bash
# Allocate 256 MB of huge pages (128 x 2 MB)
echo 128 | sudo tee /proc/sys/vm/nr_hugepages

# Make persistent across reboots
sudo tee -a /etc/sysctl.conf <<EOF
vm.nr_hugepages = 128
EOF
```

#### Optimize Network Interface

```bash
# Increase RX/TX ring sizes
sudo ethtool -G eth0 rx 4096 tx 4096

# Enable multi-queue (match number of CPU cores)
sudo ethtool -L eth0 combined 4

# Configure interrupt coalescing (balanced)
sudo ethtool -C eth0 rx-usecs 10 tx-usecs 10

# Enable offloads
sudo ethtool -K eth0 rx on tx on sg on tso on gso on gro on

# Disable features incompatible with XDP (if needed)
# sudo ethtool -K eth0 lro off rxvlan off txvlan off

# Make persistent (create /etc/network/if-up.d/ethtool-eth0)
sudo tee /etc/network/if-up.d/ethtool-eth0 <<'EOF'
#!/bin/bash
ethtool -G eth0 rx 4096 tx 4096
ethtool -L eth0 combined 4
ethtool -C eth0 rx-usecs 10 tx-usecs 10
ethtool -K eth0 rx on tx on sg on tso on gso on gro on
EOF
sudo chmod +x /etc/network/if-up.d/ethtool-eth0
```

#### Configure IRQ Affinity (Optional)

```bash
# Find IRQ numbers for eth0
grep eth0 /proc/interrupts | awk '{print $1}' | tr -d ':'

# Pin IRQs to specific CPUs (example: IRQs 30-33 to CPUs 0-3)
for i in {0..3}; do
  irq=$((30 + i))
  cpu=$i
  echo $cpu | sudo tee /proc/irq/$irq/smp_affinity_list
done
```

### Step 4: Create Configuration

```bash
# Create config directory
mkdir -p ~/.config/wraith

# Generate configuration
wraith daemon --print-config > ~/.config/wraith/config.toml

# Edit configuration
$EDITOR ~/.config/wraith/config.toml
```

**Example configuration** (`~/.config/wraith/config.toml`):

```toml
[node]
# Leave empty to generate random identity on first run
# Or specify path to encrypted private key
# private_key_file = "/home/user/.wraith/private_key"

[network]
listen_addr = "0.0.0.0:40000"
enable_xdp = true
xdp_interface = "eth0"
udp_fallback = true  # Fallback to UDP if XDP fails

[obfuscation]
default_level = "medium"
tls_mimicry = true
cover_traffic = false

[discovery]
bootstrap_nodes = [
    "bootstrap1.example.com:40000",
    "bootstrap2.example.com:40000",
]
relay_servers = [
    "relay1.example.com:41000",
    "relay2.example.com:41000",
]

[transfer]
chunk_size = 262144  # 256 KB
max_concurrent = 10
enable_resume = true

[logging]
level = "info"
# file = "/var/log/wraith/wraith.log"  # Optional log file
```

### Step 5: Generate Identity

```bash
# Generate Ed25519 keypair
wraith keygen --output ~/.wraith/private_key

# Keypair will be encrypted with passphrase
# Public key (Node ID) will be displayed
```

### Step 6: Verify XDP

```bash
# Start WRAITH daemon with debug logging
wraith daemon --bind 0.0.0.0:40000 --verbose

# Expected output:
# [INFO] WRAITH Daemon v0.9.0
# [INFO] Node ID: <hex-encoded-public-key>
# [INFO] XDP enabled on interface eth0
# [INFO] AF_XDP socket bound to 0.0.0.0:40000
# [INFO] Zero-copy mode: enabled
# [INFO] Listening for connections...
```

If XDP is unavailable:
```
# [WARN] XDP unavailable: falling back to UDP
# [WARN] Reason: <error-message>
# [INFO] UDP socket bound to 0.0.0.0:40000
```

Check troubleshooting section for common XDP errors.

### Step 7: Create Systemd Service

```bash
# Create systemd unit file
sudo tee /etc/systemd/system/wraith.service <<EOF
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
PrivateTmp=true
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/wraith /var/log/wraith

# Resource limits
LimitNOFILE=65536
LimitMEMLOCK=infinity

[Install]
WantedBy=multi-user.target
EOF

# Create wraith user
sudo useradd -r -s /bin/false wraith

# Grant capabilities to wraith user
sudo setcap \
  cap_net_raw=ep,cap_net_admin=ep,cap_bpf=ep,cap_ipc_lock=ep \
  /usr/local/bin/wraith

# Create directories
sudo mkdir -p /var/lib/wraith /var/log/wraith
sudo chown wraith:wraith /var/lib/wraith /var/log/wraith

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable wraith
sudo systemctl start wraith

# Check status
sudo systemctl status wraith
```

### Step 8: Configure Firewall

```bash
# Allow WRAITH traffic (port 40000)
sudo ufw allow 40000/udp comment "WRAITH Protocol"

# Or with iptables
sudo iptables -A INPUT -p udp --dport 40000 -j ACCEPT
sudo iptables-save | sudo tee /etc/iptables/rules.v4
```

### Step 9: Monitoring

```bash
# Monitor logs
journalctl -u wraith -f

# Or with traditional syslog
tail -f /var/log/wraith/wraith.log

# Check metrics
wraith metrics  # (future feature)
```

## Production Deployments

### High-Availability Setup

Deploy multiple WRAITH nodes with failover:

```bash
# Node 1 (primary)
wraith daemon --bind 10.0.0.1:40000

# Node 2 (secondary)
wraith daemon --bind 10.0.0.2:40000

# Configure keepalived for VIP failover
sudo tee /etc/keepalived/keepalived.conf <<EOF
vrrp_instance WRAITH {
    state MASTER
    interface eth0
    virtual_router_id 51
    priority 100  # Higher = preferred
    advert_int 1
    virtual_ipaddress {
        10.0.0.100/24
    }
}
EOF
```

### Load Balancing

Use DNS round-robin or ECMP for load distribution:

```bash
# DNS round-robin (update DNS records)
wraith1.example.com.    IN  A   10.0.0.1
wraith1.example.com.    IN  A   10.0.0.2
wraith1.example.com.    IN  A   10.0.0.3

# Or ECMP routing (requires BGP/OSPF)
ip route add 10.0.1.0/24 \
  nexthop via 10.0.0.1 dev eth0 weight 1 \
  nexthop via 10.0.0.2 dev eth0 weight 1 \
  nexthop via 10.0.0.3 dev eth0 weight 1
```

### Scaling

**Vertical Scaling** (single node):
- Increase CPU cores (up to NIC queue limit)
- Increase memory (for larger UMEM)
- Upgrade to faster NIC (10G → 25G → 100G)

**Horizontal Scaling** (multiple nodes):
- Deploy across multiple hosts
- Use DHT for peer discovery
- Implement consistent hashing for load distribution

### Multi-Datacenter

Deploy WRAITH across datacenters:

```bash
# DC1: Bootstrap nodes
dc1-bootstrap1.example.com:40000
dc1-bootstrap2.example.com:40000

# DC2: Bootstrap nodes
dc2-bootstrap1.example.com:40000
dc2-bootstrap2.example.com:40000

# Configure cross-DC discovery
[discovery]
bootstrap_nodes = [
    "dc1-bootstrap1.example.com:40000",
    "dc1-bootstrap2.example.com:40000",
    "dc2-bootstrap1.example.com:40000",
    "dc2-bootstrap2.example.com:40000",
]
```

## Cloud Deployments

### AWS EC2

```bash
# Launch instance (C5n for enhanced networking)
aws ec2 run-instances \
  --instance-type c5n.2xlarge \
  --image-id ami-0c55b159cbfafe1f0 \
  --key-name my-key \
  --security-group-ids sg-0123456789 \
  --subnet-id subnet-0123456789

# Enable enhanced networking (ENA)
aws ec2 modify-instance-attribute \
  --instance-id i-0123456789 \
  --ena-support

# SSH and deploy WRAITH
ssh ubuntu@<instance-ip>
# Follow deployment steps above

# Note: ENA supports XDP copy mode only (no zero-copy)
```

### Google Cloud Platform (GCP)

```bash
# Create instance (C2 for compute-optimized)
gcloud compute instances create wraith-node \
  --machine-type c2-standard-4 \
  --image-family ubuntu-2204-lts \
  --image-project ubuntu-os-cloud \
  --boot-disk-size 50GB

# Enable gVNIC (for XDP support)
gcloud compute instances update wraith-node \
  --network-interface nic-type=GVNIC

# SSH and deploy WRAITH
gcloud compute ssh wraith-node
# Follow deployment steps above

# Note: gVNIC supports XDP copy mode only (no zero-copy)
```

### Microsoft Azure

```bash
# Create VM (Dasv5 for accelerated networking)
az vm create \
  --resource-group wraith-rg \
  --name wraith-node \
  --image Ubuntu2204 \
  --size Standard_D4as_v5 \
  --accelerated-networking true \
  --admin-username azureuser \
  --ssh-key-values ~/.ssh/id_rsa.pub

# Enable accelerated networking (SR-IOV)
az network nic update \
  --resource-group wraith-rg \
  --name wraith-nodeVMNic \
  --accelerated-networking true

# SSH and deploy WRAITH
ssh azureuser@<vm-ip>
# Follow deployment steps above

# Note: Azure SR-IOV with Mellanox supports zero-copy XDP!
```

**Recommendation**: Use Azure with accelerated networking for best cloud XDP performance.

## Container Deployments

### Docker

```dockerfile
# Dockerfile
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl build-essential pkg-config

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy WRAITH source
WORKDIR /app
COPY . .

# Build WRAITH
RUN cargo build --release --bin wraith

# Grant capabilities
RUN setcap \
    cap_net_raw=ep,cap_net_admin=ep,cap_bpf=ep,cap_ipc_lock=ep \
    /app/target/release/wraith

# Expose port
EXPOSE 40000/udp

# Run daemon
CMD ["/app/target/release/wraith", "daemon", "--bind", "0.0.0.0:40000"]
```

**Run container**:
```bash
# Build image
docker build -t wraith:latest .

# Run with host networking (required for XDP)
docker run -d \
  --name wraith-daemon \
  --network host \
  --cap-add NET_RAW \
  --cap-add NET_ADMIN \
  --cap-add BPF \
  --cap-add IPC_LOCK \
  --ulimit memlock=-1:-1 \
  -v /var/lib/wraith:/var/lib/wraith \
  wraith:latest
```

**Note**: Container must use `--network host` for XDP support. Bridge networking breaks zero-copy.

### Kubernetes

```yaml
# wraith-deployment.yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: wraith
  namespace: wraith-system
spec:
  selector:
    matchLabels:
      app: wraith
  template:
    metadata:
      labels:
        app: wraith
    spec:
      hostNetwork: true  # Required for XDP
      containers:
      - name: wraith
        image: wraith:latest
        securityContext:
          capabilities:
            add:
            - NET_RAW
            - NET_ADMIN
            - BPF
            - IPC_LOCK
        resources:
          limits:
            memory: 2Gi
            cpu: 2000m
          requests:
            memory: 1Gi
            cpu: 1000m
        volumeMounts:
        - name: wraith-data
          mountPath: /var/lib/wraith
      volumes:
      - name: wraith-data
        hostPath:
          path: /var/lib/wraith
          type: DirectoryOrCreate
```

Apply:
```bash
kubectl apply -f wraith-deployment.yaml
```

## Monitoring and Logging

### Prometheus Metrics (Future Feature)

```bash
# Expose Prometheus metrics endpoint
wraith daemon --metrics-addr 0.0.0.0:9090

# Scrape with Prometheus
# Add to prometheus.yml:
scrape_configs:
  - job_name: 'wraith'
    static_configs:
      - targets: ['localhost:9090']
```

### Logging

Configure structured logging:

```toml
[logging]
level = "info"  # trace, debug, info, warn, error
format = "json"  # json or text
file = "/var/log/wraith/wraith.log"
max_size = "100MB"
max_backups = 10
compress = true
```

### Health Checks

```bash
# Basic health check (future feature)
wraith health

# Expected output:
# Status: healthy
# Uptime: 24h 15m 32s
# Active sessions: 42
# Active transfers: 7
# XDP mode: zero-copy
# Throughput: 8.5 Gbps (in) / 9.2 Gbps (out)
```

## Security Best Practices

### 1. Principle of Least Privilege

- Use capabilities instead of root
- Run as dedicated user (`wraith`)
- Restrict file permissions (0600 for configs, 0400 for keys)

### 2. Network Isolation

- Deploy in isolated network segment
- Use firewall rules to restrict access
- Disable unnecessary services

### 3. Encryption

- Always encrypt private keys with passphrases
- Use strong passphrases (16+ characters, random)
- Store passphrases in secure vaults (e.g., HashiCorp Vault)

### 4. Audit Logging

- Enable audit logs for all operations
- Ship logs to centralized SIEM (Security Information and Event Management)
- Set up alerts for suspicious activity

### 5. Updates

- Subscribe to WRAITH security advisories
- Test updates in staging before production
- Automate security patch deployment

## Troubleshooting Deployment Issues

### Issue: XDP Initialization Failed

**Error**: `AF_XDP socket creation failed`

**Diagnosis**:
```bash
# Check kernel support
zgrep CONFIG_XDP_SOCKETS /proc/config.gz

# Check capabilities
getcap /usr/local/bin/wraith

# Check locked memory limit
ulimit -l
```

**Solutions**:
- Missing kernel support: Upgrade kernel or rebuild with XDP
- Missing capabilities: Grant capabilities with `setcap`
- Insufficient memory: Set `ulimit -l unlimited`

### Issue: Zero-Copy Not Enabled

**Error**: `Zero-copy mode: disabled`

**Diagnosis**:
```bash
# Check NIC driver
ethtool -i eth0 | grep driver

# Check XDP mode
ip link show eth0 | grep xdp
```

**Solutions**:
- Wrong driver: Use ixgbe/i40e/ice/mlx5
- Copy mode fallback: XDP still works, just slower

### Issue: Performance Lower Than Expected

**Diagnosis**:
```bash
# Check packet drops
ethtool -S eth0 | grep drop

# Check CPU usage
mpstat -P ALL 1 5
```

**Solutions**:
- See [Performance Guide](performance.md) for optimization

## References

- [WRAITH User Guide](../USER_GUIDE.md)
- [WRAITH Configuration Reference](../CONFIG_REFERENCE.md)
- [XDP Requirements](requirements.md)
- [XDP Performance](performance.md)
- [XDP Troubleshooting](troubleshooting.md)

---

**Last Updated**: 2025-12-06
**WRAITH Version**: 0.9.0 Beta
**Sprint**: 11.5 - XDP Documentation & CLI
