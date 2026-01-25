# WRAITH Integration Templates

This directory contains deployment and integration templates for running WRAITH Protocol in production environments.

## Templates

### docker-compose.yml

Docker Compose configuration for running a WRAITH node in a container.

**Quick Start:**
```bash
docker-compose up -d
```

**Features:**
- UDP and TCP port exposure (9000)
- Persistent data volumes
- Health checks
- Environment variable configuration
- Automatic restart on failure

**Customization:**
```yaml
environment:
  - WRAITH_LOG_LEVEL=debug    # Increase logging
  - WRAITH_DHT_ENABLED=false  # Disable DHT discovery
```

### wraith.service

Systemd service unit for running WRAITH as a system service on Linux.

**Installation:**
```bash
# Create service user
sudo useradd -r -s /sbin/nologin wraith
sudo mkdir -p /var/lib/wraith /var/log/wraith
sudo chown wraith:wraith /var/lib/wraith /var/log/wraith

# Install service
sudo cp wraith.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now wraith
```

**Management:**
```bash
sudo systemctl status wraith   # Check status
sudo systemctl restart wraith  # Restart service
sudo journalctl -u wraith -f   # View logs
```

**Security Features:**
- Runs as dedicated non-root user
- Filesystem protection (`ProtectSystem=strict`)
- Private temp directory
- Read-only home directory access
- Restricted write paths

## Kubernetes Deployment

For Kubernetes deployments, create a ConfigMap from the node configuration:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: wraith-config
data:
  config.json: |
    {
      "node": {
        "name": "k8s-wraith-node",
        "listen_port": 9000
      },
      "discovery": {
        "dht_enabled": true
      }
    }
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: wraith-node
spec:
  replicas: 1
  selector:
    matchLabels:
      app: wraith-node
  template:
    metadata:
      labels:
        app: wraith-node
    spec:
      containers:
      - name: wraith
        image: ghcr.io/doublegate/wraith-protocol:latest
        ports:
        - containerPort: 9000
          protocol: UDP
        - containerPort: 9000
          protocol: TCP
        volumeMounts:
        - name: config
          mountPath: /config
      volumes:
      - name: config
        configMap:
          name: wraith-config
```

## Nginx Reverse Proxy

For TCP connections through a reverse proxy:

```nginx
stream {
    upstream wraith_backend {
        server 127.0.0.1:9000;
    }

    server {
        listen 443;
        proxy_pass wraith_backend;
        proxy_timeout 3s;
        proxy_connect_timeout 1s;
    }
}
```

## Related Documentation

- Main templates: [../README.md](../README.md)
- Configuration templates: [../config/README.md](../config/README.md)
- Operations guide: `/docs/operations/`
