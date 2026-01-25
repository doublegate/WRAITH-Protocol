# WRAITH Protocol Templates

This directory contains configuration and documentation templates for the WRAITH Protocol ecosystem. Templates provide ready-to-use starting points for common deployment and configuration scenarios.

## Directory Structure

```
templates/
├── README.md                    # This file
├── recon/                       # WRAITH-Recon ROE templates
│   ├── roe-minimal.json         # Minimal internal assessment
│   ├── roe-standard.json        # Standard penetration test
│   ├── roe-comprehensive.json   # Enterprise engagement
│   ├── roe-ctf.json             # CTF competition
│   ├── roe-bug-bounty.json      # Bug bounty program
│   ├── roe-red-team.json        # Red team exercise
│   ├── roe-schema.json          # JSON Schema validation
│   └── README.md                # ROE documentation
├── config/                      # Configuration templates
│   ├── wraith-config.toml       # CLI configuration
│   ├── node-config.json         # Node configuration
│   └── README.md                # Config documentation
├── transfer/                    # Transfer profile templates
│   ├── transfer-profile.json    # Transfer settings
│   └── README.md                # Transfer documentation
└── integration/                 # Deployment templates
    ├── docker-compose.yml       # Docker deployment
    ├── wraith.service           # Systemd service unit
    └── README.md                # Integration documentation
```

## Quick Start

### Configuration Setup

```bash
# Copy and customize CLI configuration
mkdir -p ~/.config/wraith
cp templates/config/wraith-config.toml ~/.config/wraith/config.toml
```

### Docker Deployment

```bash
# Deploy with Docker Compose
cd templates/integration
docker-compose up -d
```

### Systemd Service

```bash
# Install systemd service (Linux)
sudo cp templates/integration/wraith.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now wraith
```

### Security Assessment ROE

```bash
# Copy and customize ROE template
cp templates/recon/roe-standard.json my-engagement.json
# Edit my-engagement.json
# Sign with wraith-recon roe sign ...
```

## Template Categories

### Recon Templates (`recon/`)

Rules of Engagement (ROE) templates for WRAITH-Recon security assessments:

| Template | Use Case | Complexity |
|----------|----------|------------|
| `roe-minimal.json` | Internal scans, lab testing | Low |
| `roe-standard.json` | Penetration tests | Medium |
| `roe-comprehensive.json` | Enterprise assessments | High |
| `roe-ctf.json` | CTF competitions | Variable |
| `roe-bug-bounty.json` | Bug bounty programs | Medium |
| `roe-red-team.json` | Adversary simulation | High |

All ROE templates require cryptographic signing before use.

### Configuration Templates (`config/`)

Node and CLI configuration files:

| Template | Format | Purpose |
|----------|--------|---------|
| `wraith-config.toml` | TOML | CLI and daemon settings |
| `node-config.json` | JSON | Programmatic node setup |

### Transfer Templates (`transfer/`)

File transfer profile configurations:

| Template | Purpose |
|----------|---------|
| `transfer-profile.json` | Default transfer settings |

Customize for different network conditions (LAN, WAN, mobile).

### Integration Templates (`integration/`)

Deployment and infrastructure templates:

| Template | Platform | Purpose |
|----------|----------|---------|
| `docker-compose.yml` | Docker | Container deployment |
| `wraith.service` | Linux/systemd | System service |

## Validation

### JSON Schema Validation

Validate JSON templates with jsonschema:

```bash
# Install jsonschema
pip install jsonschema

# Validate ROE document
jsonschema --instance my-roe.json templates/recon/roe-schema.json

# Validate JSON syntax
python3 -c "import json; json.load(open('my-config.json'))"
```

### TOML Validation

```bash
# Validate TOML syntax
python3 -c "import tomllib; tomllib.load(open('config.toml', 'rb'))"
```

### YAML Validation

```bash
# Validate YAML syntax
python3 -c "import yaml; yaml.safe_load(open('docker-compose.yml'))"
```

## Customization Guidelines

### 1. Always Customize Before Use

Templates contain placeholder values (e.g., `example.test`, `operator-001`). Replace all placeholders with real values.

### 2. Security Considerations

- Never commit sensitive data (keys, passwords) to version control
- Use environment variables for secrets in production
- Review and restrict network scope appropriately
- Sign ROE documents with proper authorization

### 3. Version Control

Keep customized templates in your own repository or configuration management system. Reference these templates as starting points, not authoritative sources.

## Related Documentation

| Topic | Location |
|-------|----------|
| API Reference | `/docs/engineering/API_REFERENCE.md` |
| Security Guide | `/docs/security/` |
| Operations Guide | `/docs/operations/` |
| Client Applications | `/docs/clients/` |
| Architecture Overview | `/docs/architecture/` |

## Contributing

To add new templates:

1. Create the template file with realistic example values
2. Add a README.md in the template category directory
3. Update this README.md with the new template
4. Validate all JSON/YAML/TOML syntax
5. Submit a pull request

## License

Templates are provided under the same license as the WRAITH Protocol project. See `/LICENSE` for details.
