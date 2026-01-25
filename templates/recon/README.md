# WRAITH-Recon ROE Templates

This directory contains Rules of Engagement (RoE) templates for WRAITH-Recon security assessments. These templates provide starting points for different types of engagements and should be customized for your specific needs.

## Important Security Notice

**All RoE documents must be cryptographically signed before use.** WRAITH-Recon will refuse to operate without a valid Ed25519 signature. The `signer_public_key` and `signature` fields must contain valid hex-encoded values.

## Template Overview

| Template | Use Case | Scope Level |
|----------|----------|-------------|
| `roe-minimal.json` | Quick internal assessments | Minimal |
| `roe-standard.json` | Typical penetration tests | Standard |
| `roe-comprehensive.json` | Formal enterprise engagements | Full |
| `roe-ctf.json` | Capture The Flag competitions | CTF-specific |
| `roe-bug-bounty.json` | Bug bounty program participation | Limited |
| `roe-red-team.json` | Advanced adversary simulation | Full Red Team |

## Template Descriptions

### roe-minimal.json

A minimal template with only essential fields for quick internal security assessments. Best for:
- Internal network scans
- Quick vulnerability assessments
- Lab/development environment testing
- Proof-of-concept testing

### roe-standard.json

A balanced template for typical penetration testing engagements. Includes:
- Multiple authorized operators
- Network and domain scope
- Common MITRE ATT&CK techniques
- Data exfiltration limits
- Emergency contacts and constraints

Best for:
- External/internal penetration tests
- Vulnerability assessments
- Security audits
- Compliance testing (PCI-DSS, SOC 2)

### roe-comprehensive.json

A full-featured template for formal enterprise engagements. Includes:
- Extended team authorization
- Comprehensive scope definition
- Detailed technique allowlists
- Strict data handling limits
- Multiple emergency contacts
- Detailed operational constraints
- Regulatory compliance considerations

Best for:
- Enterprise security assessments
- Financial sector engagements
- Healthcare security testing
- Government/defense assessments
- Multi-phase engagements

### roe-ctf.json

Specialized template for Capture The Flag competitions. Features:
- Permissive technique authorization
- Infrastructure exclusions
- Time-limited engagement window
- Minimal constraints (competition focus)

Best for:
- CTF events
- Training exercises
- Security competitions
- Lab environments

### roe-bug-bounty.json

Template for bug bounty program participation. Includes:
- Domain-only scope (no CIDR)
- Conservative technique selection
- Strict data handling limits
- Program-specific constraints

Best for:
- Public bug bounty programs
- Vulnerability disclosure programs
- Coordinated disclosure

### roe-red-team.json

Advanced template for full adversary simulation exercises. Features:
- Comprehensive technique authorization
- Social engineering and physical testing scope
- White Team coordination requirements
- Extended engagement windows
- Detailed deconfliction procedures

Best for:
- Red Team exercises
- APT simulation
- Purple Team engagements
- Adversary emulation

## Quick Start

### 1. Choose a Template

Select the template that best matches your engagement type.

### 2. Customize the Template

```bash
cp roe-standard.json my-engagement-roe.json
# Edit my-engagement-roe.json with your details
```

### 3. Sign the Document

Sign the RoE document using the WRAITH-Recon CLI:

```bash
wraith-recon roe sign --key signing-key.pem --input my-engagement-roe.json --output my-engagement-roe-signed.json
```

### 4. Load into WRAITH-Recon

```bash
wraith-recon --roe my-engagement-roe-signed.json
```

## Schema Validation

Validate your RoE document against the schema:

```bash
# Using Python jsonschema (pip install jsonschema)
jsonschema --instance my-engagement-roe.json roe-schema.json
```

## Related Documentation

- Main templates documentation: [../README.md](../README.md)
- WRAITH-Recon client: `/docs/clients/WRAITH-Recon.md`
- Security testing parameters: `/ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md`

## Legal Notice

These templates are provided as starting points only. You are responsible for ensuring your RoE documents comply with applicable laws, regulations, and contractual requirements. Always obtain proper written authorization before conducting security assessments.
