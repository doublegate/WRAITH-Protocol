# WRAITH Transfer Templates

This directory contains configuration templates for WRAITH file transfer operations.

## Templates

### transfer-profile.json

Transfer profile template for customizing file transfer behavior. Profiles allow you to save and reuse transfer configurations.

**Key Settings:**

| Setting | Description | Default |
|---------|-------------|---------|
| `chunk_size` | File chunk size in bytes | 1 MiB |
| `compression` | Compression algorithm (`none`, `zstd`, `lz4`) | `zstd` |
| `compression_level` | Compression level (1-22 for zstd) | 3 |
| `encryption` | Encryption algorithm | `xchacha20-poly1305` |
| `verify_integrity` | Verify chunks after transfer | `true` |
| `resume_enabled` | Allow resuming interrupted transfers | `true` |

**Bandwidth Settings:**

| Setting | Description | Default |
|---------|-------------|---------|
| `max_upload_kbps` | Upload limit in KB/s (0 = unlimited) | 0 |
| `max_download_kbps` | Download limit in KB/s (0 = unlimited) | 0 |
| `throttle_on_battery` | Reduce bandwidth on battery power | `true` |

**Retry Settings:**

| Setting | Description | Default |
|---------|-------------|---------|
| `max_attempts` | Maximum retry attempts | 5 |
| `initial_delay_ms` | Initial retry delay | 1000 ms |
| `max_delay_ms` | Maximum retry delay | 30000 ms |
| `backoff_multiplier` | Exponential backoff multiplier | 2.0 |

## Example Profiles

### High-Speed LAN Transfer
```json
{
  "profile": { "name": "lan-fast", "description": "Optimized for local network" },
  "settings": {
    "chunk_size": 4194304,
    "compression": "none",
    "verify_integrity": true
  },
  "bandwidth": { "max_upload_kbps": 0, "max_download_kbps": 0 }
}
```

### Bandwidth-Limited WAN Transfer
```json
{
  "profile": { "name": "wan-limited", "description": "Constrained bandwidth" },
  "settings": {
    "chunk_size": 524288,
    "compression": "zstd",
    "compression_level": 9
  },
  "bandwidth": { "max_upload_kbps": 5120, "max_download_kbps": 10240 }
}
```

### Mobile/Metered Connection
```json
{
  "profile": { "name": "mobile", "description": "Mobile data optimization" },
  "settings": {
    "chunk_size": 262144,
    "compression": "zstd",
    "compression_level": 15
  },
  "bandwidth": {
    "max_upload_kbps": 1024,
    "max_download_kbps": 2048,
    "throttle_on_battery": true
  }
}
```

## Usage

### CLI
```bash
wraith transfer --profile transfer-profile.json send file.zip peer-id
```

### WRAITH-Transfer App
Import profiles via Settings > Transfer Profiles > Import.

## Related Documentation

- Main templates: [../README.md](../README.md)
- WRAITH-Transfer client: `/docs/clients/WRAITH-Transfer.md`
- File transfer protocol: `/docs/technical/FILE_TRANSFER.md`
