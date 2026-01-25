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

## Field Reference

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique document identifier (e.g., ROE-2026-STD-001) |
| `version` | string | Document version (semantic versioning) |
| `organization` | string | Assessing organization name |
| `title` | string | Engagement title |
| `authorized_operators` | array | List of authorized operator IDs |
| `start_time` | datetime | Engagement start (RFC 3339 format) |
| `end_time` | datetime | Engagement end (RFC 3339 format) |
| `signer_public_key` | string | Hex-encoded Ed25519 public key (64 chars) |
| `signature` | string | Hex-encoded Ed25519 signature (128 chars) |

### Optional Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `description` | string | "" | Detailed engagement description |
| `client_name` | string | "" | Target organization name |
| `authorized_cidrs` | array | [] | Authorized networks (CIDR notation) |
| `authorized_domains` | array | [] | Authorized domains (wildcards supported) |
| `excluded_targets` | array | [] | Explicitly excluded targets |
| `authorized_techniques` | array | [] | Allowed MITRE ATT&CK IDs (empty = all allowed) |
| `prohibited_techniques` | array | [] | Prohibited MITRE ATT&CK IDs |
| `max_exfil_rate` | int/null | null | Max exfiltration rate (bytes/sec) |
| `max_exfil_total` | int/null | null | Max total exfiltration (bytes) |
| `emergency_contacts` | array | [] | Emergency contact list |
| `constraints` | array | [] | Additional operational constraints |
| `created_at` | datetime | - | Document creation timestamp |

## MITRE ATT&CK Technique Reference

### Common Reconnaissance Techniques

| ID | Name | Description |
|----|------|-------------|
| T1595 | Active Scanning | Network scanning, vulnerability scanning |
| T1592 | Gather Victim Host Information | OS, software, configuration discovery |
| T1046 | Network Service Discovery | Port scanning, service enumeration |
| T1040 | Network Sniffing | Passive traffic capture |
| T1018 | Remote System Discovery | Network host enumeration |
| T1016 | System Network Configuration Discovery | Network settings |
| T1082 | System Information Discovery | System details |
| T1083 | File and Directory Discovery | File system enumeration |

### Commonly Prohibited Techniques

| ID | Name | Reason |
|----|------|--------|
| T1486 | Data Encrypted for Impact | Ransomware behavior |
| T1485 | Data Destruction | Destructive attack |
| T1561 | Disk Wipe | Destructive attack |
| T1490 | Inhibit System Recovery | Destructive attack |
| T1529 | System Shutdown/Reboot | Denial of service |
| T1499 | Endpoint Denial of Service | DoS attack |
| T1498 | Network Denial of Service | DDoS attack |

See [MITRE ATT&CK](https://attack.mitre.org/) for the complete technique reference.

## How to Use

### 1. Choose a Template

Select the template that best matches your engagement type.

### 2. Customize the Template

Edit the template to match your specific requirements:

```bash
cp roe-standard.json my-engagement-roe.json
# Edit my-engagement-roe.json with your details
```

### 3. Generate Signing Keys

Generate an Ed25519 keypair for signing:

```bash
# Using OpenSSL
openssl genpkey -algorithm ED25519 -out roe-signing-key.pem
openssl pkey -in roe-signing-key.pem -pubout -out roe-signing-key.pub
```

### 4. Sign the Document

Sign the RoE document using the WRAITH-Recon CLI or a compatible signing tool:

```bash
# Using WRAITH-Recon CLI (when available)
wraith-recon roe sign --key roe-signing-key.pem --input my-engagement-roe.json --output my-engagement-roe-signed.json
```

### 5. Load into WRAITH-Recon

Load the signed RoE into WRAITH-Recon:

```bash
wraith-recon --roe my-engagement-roe-signed.json
```

## Validation

Validate your RoE document against the schema:

```bash
# Using Python
python3 -c "import json; json.load(open('my-engagement-roe.json'))"

# Using jsonschema (pip install jsonschema)
jsonschema --instance my-engagement-roe.json roe-schema.json
```

## Best Practices

1. **Always use unique IDs** - Use a consistent naming scheme (e.g., ROE-YYYY-TYPE-NNN)

2. **Set appropriate time windows** - Use conservative engagement windows that match the actual testing period

3. **Define explicit exclusions** - Always explicitly exclude production systems, critical infrastructure, and out-of-scope targets

4. **Include emergency contacts** - Ensure multiple contacts are available 24/7 during the engagement

5. **Document constraints** - Be specific about operational limitations and required notifications

6. **Limit data handling** - Set appropriate exfiltration limits to prevent accidental data exposure

7. **Use technique allowlists** - For sensitive engagements, explicitly list allowed techniques rather than relying on prohibitions alone

8. **Keep prohibited techniques** - Always prohibit destructive techniques (ransomware, data destruction, DoS)

9. **Sign with proper keys** - Use keys that are securely stored and properly authorized

10. **Version control** - Track RoE document versions and maintain change history

## Schema Validation

The `roe-schema.json` file provides JSON Schema validation for RoE documents. Use it to validate your customized templates before signing.

## Support

For questions about RoE requirements or WRAITH-Recon configuration:

- Documentation: `/docs/clients/WRAITH-Recon.md`
- Issue Tracker: GitHub Issues
- Security Testing Parameters: `/ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md`

## Legal Notice

These templates are provided as starting points only. You are responsible for ensuring your RoE documents comply with applicable laws, regulations, and contractual requirements. Always obtain proper written authorization before conducting security assessments.
