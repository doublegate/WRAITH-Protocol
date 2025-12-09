# RFC-Required Cryptographic Algorithms

## Overview

This document explains the use of weak cryptographic algorithms (MD5, SHA1) in
WRAITH Protocol where they are mandated by external protocol specifications.

## STUN Protocol (RFC 5389)

### Background

The STUN (Session Traversal Utilities for NAT) protocol is defined by RFC 5389
and is used for NAT traversal and discovering server reflexive addresses. WRAITH
uses STUN for peer discovery and connectivity.

### Mandated Algorithms

RFC 5389 Section 15.4 specifically requires:

1. **HMAC-SHA1** for MESSAGE-INTEGRITY attribute computation
2. **MD5** for long-term credential key derivation

These requirements are part of the protocol specification and cannot be changed
without breaking compatibility with other STUN implementations.

### Security Context

While MD5 and SHA1 are considered cryptographically weak for collision resistance,
their use in STUN is acceptable because:

1. **Limited Scope**: Used only for STUN protocol operations, not for general
   cryptographic purposes
2. **HMAC Construction**: HMAC-SHA1 provides better security properties than
   plain SHA1 due to the keyed-hash design
3. **Protocol-Level Security**: STUN includes additional security mechanisms like
   transaction ID validation and fingerprint verification
4. **Standard Compliance**: Required for interoperability with all STUN servers
   and clients

### Implementation Location

- **Module**: `crates/wraith-discovery/src/nat/stun.rs`
- **Dependencies**: `md-5 = "0.10"`, `sha1 = "0.10"`, `hmac = "0.12"`
- **Usage**:
  - `StunAuthentication::derive_key()` - MD5 for long-term credentials
  - `StunMessage::add_message_integrity()` - HMAC-SHA1 for message integrity
  - `StunMessage::verify_message_integrity()` - HMAC-SHA1 verification

## Mitigation Strategy

To prevent misuse of weak algorithms:

1. **Isolated Usage**: MD5 and SHA1 are only imported in the STUN module
2. **Documentation**: Extensive comments warn against using these algorithms
   for other purposes
3. **Strong Alternatives**: WRAITH's general cryptography (in `wraith-crypto`)
   uses modern algorithms:
   - BLAKE3 for hashing
   - ChaCha20-Poly1305 for AEAD encryption
   - Ed25519 for signatures
   - X25519 for key exchange

## Code Scanning Alerts

Code scanning tools (CodeQL, etc.) may flag the use of MD5 and SHA1 as security
vulnerabilities. These alerts are **expected and acceptable** for the STUN
implementation because:

1. The algorithms are mandated by RFC 5389
2. Their use is properly documented and justified
3. They are isolated to protocol-specific code
4. Alternative algorithms would break STUN compatibility

### Alert Suppression

If code scanning alerts need to be suppressed:

- **File**: `crates/wraith-discovery/src/nat/stun.rs`
- **Justification**: RFC 5389 compliance requirement
- **Scope**: Limited to STUN MESSAGE-INTEGRITY and credential derivation
- **Alternative**: None available while maintaining STUN compatibility

## References

- [RFC 5389: Session Traversal Utilities for NAT (STUN)](https://datatracker.ietf.org/doc/html/rfc5389)
- [RFC 5389 Section 15.4: MESSAGE-INTEGRITY](https://datatracker.ietf.org/doc/html/rfc5389#section-15.4)
- [WRAITH Security Documentation](./README.md)

## Review

This document should be reviewed whenever:

- STUN implementation is modified
- New protocols with algorithm requirements are added
- Code scanning tools are updated
- Security best practices change

Last Updated: 2025-12-09
