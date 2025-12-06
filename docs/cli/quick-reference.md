# WRAITH CLI Quick Reference

## Command Cheat Sheet

### Daemon Management

```bash
# Start daemon
wraith daemon --bind 0.0.0.0:40000

# Start with debug output
wraith --debug daemon

# Start with custom workers
wraith daemon --workers 8
```

### Identity Management

```bash
# Generate identity
wraith generate-key --output ~/.wraith/identity.key

# Generate encrypted identity
wraith generate-key --output ~/.wraith/identity.key --encrypt
```

### File Transfer

```bash
# Send file
wraith send FILE --to PEER_ID

# Send multiple files
wraith batch --to PEER_ID --files FILE1 FILE2 FILE3

# Receive files
wraith receive --output ~/Downloads
```

### Peer Management

```bash
# List peers
wraith peers

# Query DHT
wraith peers --dht-query

# Ping peer
wraith ping PEER_ID
```

### Status and Monitoring

```bash
# Show status
wraith status

# Show detailed status
wraith status --detailed

# Show transfer status
wraith status --transfer TRANSFER_ID

# Check health
wraith health

# Show metrics
wraith metrics

# Show metrics (JSON)
wraith metrics --json

# Watch metrics (live)
wraith metrics --watch

# Show node info
wraith info
```

### Configuration

```bash
# Show config
wraith config show

# Set value
wraith config set KEY VALUE

# Show as JSON
wraith config show --json
```

---

## Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--verbose` | `-v` | Debug-level logging |
| `--debug` | `-d` | Trace-level logging |
| `--help` | `-h` | Show help |
| `--version` | `-V` | Show version |
| `--config FILE` | | Custom config file |

---

## Common Workflows

### First-Time Setup

```bash
# 1. Generate identity
wraith generate-key --output ~/.wraith/identity.key --encrypt

# 2. Create config
mkdir -p ~/.wraith
cat > ~/.wraith/config.toml <<EOF
[node]
identity_file = "~/.wraith/identity.key"
bind = "0.0.0.0:40000"

[transport]
enable_xdp = true
enable_io_uring = true

[discovery]
enable_dht = true
bootstrap_nodes = ["dht.wraith.network:40000"]
EOF

# 3. Set capabilities
sudo setcap cap_net_raw=ep,cap_net_admin=ep,cap_bpf=ep,cap_ipc_lock=ep $(which wraith)

# 4. Start daemon
wraith daemon
```

### Send File

```bash
# 1. Start daemon (if not running)
wraith daemon &

# 2. Send file
wraith send report.pdf --to wraith:0123456789abcdef...
```

### Receive File

```bash
# 1. Start daemon (if not running)
wraith daemon &

# 2. Start receiving
wraith receive --output ~/Downloads/wraith
```

### Monitor Transfer

```bash
# Watch status
watch -n1 'wraith status'

# Watch metrics
wraith metrics --watch --interval 1
```

---

## Configuration Keys

### Node Settings

```toml
[node]
identity_file = "~/.wraith/identity.key"
bind = "0.0.0.0:40000"
```

### Transport Settings

```toml
[transport]
enable_xdp = true           # Kernel bypass (requires root/caps)
enable_io_uring = true      # Async file I/O (Linux only)
worker_threads = 4          # Worker threads (default: CPU count)
connection_timeout_secs = 30
idle_timeout_secs = 300
```

### Obfuscation Settings

```toml
[obfuscation]
padding_mode = "SizeClasses"       # None, PowerOfTwo, SizeClasses, ConstantRate, Statistical
timing_mode = "Normal"             # None, Fixed, Uniform, Normal, Exponential
protocol_mimicry = "TLS"           # None, TLS, WebSocket, DoH
```

### Discovery Settings

```toml
[discovery]
enable_dht = true
bootstrap_nodes = ["dht.wraith.network:40000"]
enable_nat_traversal = true
relay_servers = ["relay1.wraith.network:40000"]
```

### Transfer Settings

```toml
[transfer]
chunk_size = 262144                  # 256 KiB
max_concurrent_transfers = 10
max_concurrent_chunks = 64
download_dir = "~/Downloads/wraith"
enable_resume = true
```

### Logging Settings

```toml
[logging]
level = "Info"              # Trace, Debug, Info, Warn, Error
enable_metrics = true
```

---

## Error Codes

| Code | Description | Solution |
|------|-------------|----------|
| 1 | Permission denied | Grant capabilities or run as root |
| 2 | Connection timeout | Check network/firewall |
| 3 | Transfer failed | Retry (resume enabled) |
| 4 | Invalid configuration | Check config.toml syntax |
| 5 | Peer not found | Verify peer ID, check DHT |
| 6 | Integrity verification failed | Retry transfer |

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `WRAITH_CONFIG` | Config file path | `~/.wraith/config.toml` |
| `WRAITH_IDENTITY` | Identity file path | `~/.wraith/identity.key` |
| `WRAITH_LOG_LEVEL` | Log level | `info` |
| `WRAITH_BIND` | Bind address | `0.0.0.0:40000` |

---

## Systemd Integration

### Service File

```ini
[Unit]
Description=WRAITH Protocol Daemon
After=network.target

[Service]
Type=simple
User=wraith
ExecStart=/usr/local/bin/wraith daemon --bind 0.0.0.0:40000
Restart=on-failure
CapabilityBoundingSet=CAP_NET_RAW CAP_NET_ADMIN CAP_BPF CAP_IPC_LOCK
AmbientCapabilities=CAP_NET_RAW CAP_NET_ADMIN CAP_BPF CAP_IPC_LOCK

[Install]
WantedBy=multi-user.target
```

### Management Commands

```bash
# Enable service
sudo systemctl enable wraith

# Start service
sudo systemctl start wraith

# Stop service
sudo systemctl stop wraith

# Restart service
sudo systemctl restart wraith

# View status
sudo systemctl status wraith

# View logs
sudo journalctl -u wraith -f
```

---

## Docker Integration

### Dockerfile

```dockerfile
FROM rust:1.85-slim as builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libcap2-bin && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/wraith /usr/local/bin/
RUN setcap cap_net_raw=ep,cap_net_admin=ep,cap_bpf=ep,cap_ipc_lock=ep /usr/local/bin/wraith
EXPOSE 40000/udp
ENTRYPOINT ["wraith"]
CMD ["daemon", "--bind", "0.0.0.0:40000"]
```

### Docker Commands

```bash
# Build image
docker build -t wraith:latest .

# Run daemon
docker run -d \
  --name wraith \
  --network host \
  --cap-add NET_RAW \
  --cap-add NET_ADMIN \
  --cap-add BPF \
  --cap-add IPC_LOCK \
  -v ~/.wraith:/root/.wraith \
  wraith:latest

# View logs
docker logs -f wraith

# Stop daemon
docker stop wraith
```

---

## Performance Tuning

### Maximum Throughput

```bash
# Increase workers
wraith daemon --workers 8

# Use zero-copy mode (requires compatible NIC)
wraith config set transport.enable_xdp true

# Optimize obfuscation (less = faster)
wraith config set obfuscation.padding_mode "None"
wraith config set obfuscation.timing_mode "None"
```

### Minimum Latency

```bash
# Pin to specific cores
wraith daemon --cpus 0-3

# Disable obfuscation
wraith config set obfuscation.padding_mode "None"
wraith config set obfuscation.timing_mode "None"
wraith config set obfuscation.protocol_mimicry "None"
```

### Maximum Stealth

```bash
# Enable all obfuscation
wraith config set obfuscation.padding_mode "Statistical"
wraith config set obfuscation.timing_mode "Exponential"
wraith config set obfuscation.protocol_mimicry "TLS"
```

---

## Troubleshooting Quick Fixes

### XDP Not Working

```bash
# Grant capabilities
sudo setcap cap_net_raw=ep,cap_net_admin=ep,cap_bpf=ep,cap_ipc_lock=ep $(which wraith)

# Check kernel version (need 5.3+)
uname -r

# Check driver support
ethtool -i eth0 | grep driver
```

### Connection Issues

```bash
# Check firewall
sudo ufw allow 40000/udp

# Check NAT
wraith info | grep "NAT Status"

# Test connectivity
wraith ping PEER_ID
```

### Performance Issues

```bash
# Check zero-copy
wraith status --detailed | grep "Zero-Copy"

# Check CPU usage
wraith metrics | grep CPU

# Check packet loss
wraith status --detailed | grep "Loss"
```

---

## See Also

- [Full CLI Usage Guide](usage.md)
- [CLI Examples](examples.md)
- [XDP Troubleshooting](../xdp/troubleshooting.md)
- [Configuration Reference](../CONFIG_REFERENCE.md)

---

**Last Updated**: 2025-12-06
**WRAITH Version**: 0.9.0 Beta
