# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.x.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### How to Report

1. **Do NOT open a public GitHub issue** for security vulnerabilities
2. Send a detailed report to the maintainers via GitHub Security Advisories
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment:** Within 48 hours of your report
- **Initial Assessment:** Within 7 days
- **Resolution Timeline:** Depends on severity
  - Critical: 24-72 hours
  - High: 7 days
  - Medium: 30 days
  - Low: 90 days

### Scope

The following are in scope for security reports:

- Cryptographic weaknesses (key exchange, encryption, hashing)
- Authentication/authorization bypasses
- Information disclosure vulnerabilities
- Denial of service attacks against the protocol
- Traffic analysis vulnerabilities that break privacy guarantees
- Memory safety issues (buffer overflows, use-after-free)
- Side-channel attacks on cryptographic operations

### Out of Scope

- Social engineering attacks
- Physical attacks
- Issues in dependencies (report to upstream)
- Issues requiring unlikely user interaction

### Recognition

We appreciate security researchers who help improve WRAITH Protocol:

- Credit in release notes (with permission)
- Addition to CONTRIBUTORS.md security section
- Potential bounty for critical vulnerabilities (case-by-case)

## Security Design

WRAITH Protocol is designed with security as a core principle:

- **Cryptography:** XChaCha20-Poly1305 AEAD, X25519 key exchange, BLAKE3 hashing
- **Forward Secrecy:** Double ratchet key derivation
- **Traffic Analysis Resistance:** Elligator2 encoding, padding, timing obfuscation
- **Mutual Authentication:** Noise_XX handshake pattern
- **Memory Safety:** Rust implementation with no unsafe code in crypto paths

For detailed security architecture, see [docs/architecture/security-model.md](docs/architecture/security-model.md).
