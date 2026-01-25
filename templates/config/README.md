# WRAITH Configuration Templates

This directory contains configuration templates for WRAITH Protocol nodes and CLI.

## Templates

### wraith-config.toml

CLI and daemon configuration file template. This is the primary configuration file for WRAITH nodes.

**Installation:**
```bash
mkdir -p ~/.config/wraith
cp wraith-config.toml ~/.config/wraith/config.toml
# Edit config.toml with your settings
```

**Key Sections:**
- `[node]` - Node identity and network settings
- `[crypto]` - Key storage and hardware security module settings
- `[transfer]` - File transfer parameters
- `[network]` - Connection timeouts and retry behavior
- `[logging]` - Log level and output configuration

### node-config.json

JSON configuration for programmatic node setup. Useful for:
- Docker deployments
- Kubernetes ConfigMaps
- Automated node provisioning
- API-based configuration

**Key Sections:**
- `node` - Basic node identity and limits
- `discovery` - DHT and mDNS discovery settings
- `security` - Authentication and rate limiting
- `storage` - Data directory and cache settings

## Configuration Precedence

WRAITH configuration is loaded in the following order (later overrides earlier):

1. Built-in defaults
2. System config: `/etc/wraith/config.toml`
3. User config: `~/.config/wraith/config.toml`
4. Environment variables: `WRAITH_*`
5. Command-line arguments

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `WRAITH_LOG_LEVEL` | Override log level | `debug` |
| `WRAITH_LISTEN_ADDR` | Override listen address | `0.0.0.0:9001` |
| `WRAITH_DHT_ENABLED` | Enable/disable DHT | `true` |
| `WRAITH_KEY_PATH` | Key storage directory | `/secure/keys` |

## Related Documentation

- Main templates: [../README.md](../README.md)
- CLI reference: `/docs/engineering/API_REFERENCE.md`
- Security guide: `/docs/security/`
