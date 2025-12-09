# Security Documentation

This directory contains security-related documentation for WRAITH Protocol.

## Contents

- **[rfc-required-algorithms.md](rfc-required-algorithms.md)** - Explanation of weak cryptographic algorithms required by protocol specifications (RFC 5389 STUN)

## Code Scanning Alerts

### Handling False Positives

WRAITH Protocol undergoes automated security scanning using CodeQL and other tools.
Some alerts may appear as "vulnerabilities" when they are actually:

1. **Protocol requirements** - Algorithms mandated by RFCs for interoperability
2. **Justified design decisions** - Documented trade-offs with security rationale
3. **Tool limitations** - Scanner false positives or context misunderstandings

### Current Justified Uses

As of 2025-12-09, the following code patterns are flagged by security scanners
but are **not vulnerabilities**:

1. **MD5 usage in STUN** (`crates/wraith-discovery/src/nat/stun.rs`)
   - Required by RFC 5389 for long-term credential derivation
   - See [rfc-required-algorithms.md](rfc-required-algorithms.md)

2. **SHA1 usage in STUN** (`crates/wraith-discovery/src/nat/stun.rs`)
   - Required by RFC 5389 for HMAC-SHA1 in MESSAGE-INTEGRITY
   - See [rfc-required-algorithms.md](rfc-required-algorithms.md)

3. **HMAC-SHA1 in STUN** (`crates/wraith-discovery/src/nat/stun.rs`)
   - Required by RFC 5389 for message authentication
   - HMAC construction provides adequate security for STUN
   - See [rfc-required-algorithms.md](rfc-required-algorithms.md)

### Review Process

When a new code scanning alert is raised:

1. **Investigate** - Understand what the scanner detected
2. **Evaluate** - Determine if it's a real vulnerability or false positive
3. **Fix or Document** - Either fix the issue or document why it's justified
4. **Track** - Update this documentation with the decision

### Related Files

- `.github/codeql/codeql-config.yml` - CodeQL configuration with justifications
- `.github/workflows/codeql.yml` - CodeQL scanning workflow
- Inline comments in source files explaining security decisions

## Reporting Security Issues

If you discover a security vulnerability in WRAITH Protocol:

1. **Do not** open a public issue
2. Review our [Security Policy](../../SECURITY.md)
3. Report via the appropriate secure channel
4. Allow time for a fix before public disclosure

## Security Best Practices

For developers working on WRAITH Protocol:

1. **Use strong cryptography** - Prefer BLAKE3, ChaCha20-Poly1305, Ed25519, X25519
2. **Avoid weak algorithms** - Do not use MD5, SHA1, RC4, DES except when RFC-mandated
3. **Document exceptions** - Any use of weak algorithms must be documented and justified
4. **Validate inputs** - Sanitize paths, validate sizes, check bounds
5. **Zeroize secrets** - Clear sensitive data from memory after use
6. **Review dependencies** - Check for known vulnerabilities before adding deps

## Resources

- [OWASP Cryptographic Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html)
- [NIST Cryptographic Standards](https://csrc.nist.gov/projects/cryptographic-standards-and-guidelines)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
