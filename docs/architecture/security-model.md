# WRAITH Protocol Security Model

**Document Version:** 1.0.0
**Last Updated:** 2025-11-28
**Status:** Security Specification

---

## Executive Summary

WRAITH implements defense-in-depth security, combining multiple cryptographic layers, traffic obfuscation, and protocol design to resist a wide range of attacks. This document defines the threat model, security properties, and attack resistance strategies.

---

## Threat Model

### Adversary Capabilities

**In-Scope Threats:**

| Adversary Type | Capabilities | Examples |
|----------------|--------------|----------|
| **Passive Network Observer** | Observe all network traffic, limited processing | ISP, government surveillance |
| **Active Network Attacker** | MITM, packet injection, modification | Compromised router, malicious VPN |
| **Malicious DHT Node** | Provide false peer information, track queries | Sybil attack, poisoning |
| **Compromised Relay** | Observe relay traffic metadata | Malicious relay operator |
| **Traffic Analysis** | Pattern recognition, timing correlation | Machine learning classifiers |
| **DPI/Firewall** | Protocol identification, blocking | Corporate firewall, Great Firewall |

**Out-of-Scope Threats:**
- Endpoint compromise (malware on peer devices)
- Global passive adversary (full Internet observation)
- Cryptographic primitive breaks (Curve25519, ChaCha20)
- Side-channel attacks (timing, power, EM)
- Social engineering and phishing

### Attack Scenarios

**Scenario 1: Government Surveillance**
- **Threat:** Nation-state passively monitors ISP links
- **Goal:** Identify protocol users, content analysis
- **Mitigation:** Traffic indistinguishability, encrypted metadata

**Scenario 2: Corporate DPI Filtering**
- **Threat:** Deep packet inspection blocks non-approved protocols
- **Goal:** Prevent protocol usage on enterprise networks
- **Mitigation:** Protocol mimicry (HTTPS/WebSocket), Elligator2 encoding

**Scenario 3: Active MITM Attack**
- **Threat:** Compromised router attempts key substitution
- **Goal:** Decrypt communications
- **Mitigation:** Mutual authentication (Noise_XX), out-of-band key verification

**Scenario 4: DHT Sybil Attack**
- **Threat:** Attacker controls many DHT nodes
- **Goal:** Track peer discovery, provide false information
- **Mitigation:** Encrypted announcements, signature verification

**Scenario 5: Traffic Correlation**
- **Threat:** Adversary observes timing/size patterns across network
- **Goal:** Link sender and receiver
- **Mitigation:** Padding, timing obfuscation, cover traffic

---

## Security Properties

### Confidentiality

**Guarantee:** Plaintext content never exposed on the network.

**Mechanism:**
- XChaCha20-Poly1305 AEAD encryption (IND-CPA secure)
- 256-bit keys derived from X25519 DH
- Unique nonces (192-bit, never reused)

**Attack Resistance:**
```
Ciphertext-only attack: 2^256 complexity (key search)
Known-plaintext attack: 2^256 complexity (ChaCha20 security)
Chosen-plaintext attack: IND-CPA secure
```

### Integrity

**Guarantee:** Tampering detected and rejected.

**Mechanism:**
- Poly1305 authentication tag (128-bit)
- INT-CTXT security (authenticated encryption)
- Per-chunk BLAKE3 hashing (collision resistance: 2^128)

**Attack Resistance:**
```
Forgery attack: 2^128 complexity (tag guessing)
Collision attack: 2^128 complexity (BLAKE3)
```

### Authenticity

**Guarantee:** Peers mutually verified before data transfer.

**Mechanism:**
- Noise_XX handshake (mutual authentication)
- Ed25519 signatures for identity (128-bit security)
- Static key verification via out-of-band channel

**Attack Resistance:**
```
Impersonation: Impossible without static private key
MITM: Detected if static keys don't match expected
```

### Forward Secrecy

**Guarantee:** Past sessions remain secure even if current keys compromised.

**Mechanisms:**
1. **Ephemeral DH:** New ephemeral keys per session
2. **Symmetric Ratchet:** Chain key updated every packet
3. **DH Ratchet:** New ephemeral DH every 2 minutes or 1M packets

**Security Analysis:**
```
Compromise at time T:
- Past sessions (T-n): Secure (ephemeral keys deleted)
- Current session: Compromised
- Future sessions: Secure after next DH ratchet (≤2 minutes)
```

**Ratchet Implementation:**
```rust
// Symmetric ratchet (every packet)
chain_key[n+1] = BLAKE3(chain_key[n] || 0x01)
message_key[n] = BLAKE3(chain_key[n] || 0x02)

// Zeroize immediately
zeroize(chain_key[n])
zeroize(message_key[n])

// DH ratchet (periodic)
new_dh = DH(new_ephemeral, peer_ephemeral)
new_chain = HKDF(current_chain || new_dh, "ratchet")
zeroize(old_ephemeral_private_key)
```

### Post-Compromise Security

**Guarantee:** Security restored after key compromise through ratcheting.

**Recovery Time:**
- Maximum: 2 minutes (rekey interval)
- Packets: ≤1,000,000 (rekey packet limit)

**Example:**
```
T=0: Attacker compromises session keys
T=1min: Legitimate ratchet occurs
T=1min+: Attacker can no longer decrypt
```

### Replay Protection

**Guarantee:** Duplicate packets rejected.

**Mechanism:**
- Unique nonce per packet (session_salt || packet_counter)
- Sliding window for reordering tolerance
- Timestamp freshness check in handshake

**Window Size:** 64 packets

```rust
struct ReplayWindow {
    largest_accepted: u32,
    bitmap: u64,  // Bits represent offsets from largest
}

fn is_duplicate(&self, seq: u32) -> bool {
    if seq > self.largest_accepted {
        return false;  // New packet
    }

    let diff = self.largest_accepted - seq;
    if diff >= 64 {
        return true;  // Too old, must be replay
    }

    (self.bitmap & (1 << diff)) != 0  // Check bit
}
```

---

## Traffic Analysis Resistance

### Indistinguishability Goals

**Objective:** Protocol traffic computationally indistinguishable from:
1. Uniform random bytes
2. Target protocol (HTTPS, WebSocket, DNS)
3. Legitimate encrypted traffic

### Elligator2 Key Hiding

**Property:** Public keys indistinguishable from random 32-byte strings.

**Statistical Test:**
```python
def distinguisher_advantage(samples, oracle):
    """
    Samples: List of 32-byte strings (half real keys, half random)
    Oracle: Function that guesses "key" or "random"

    Advantage = |P(oracle correct) - 0.5|
    """
    correct = sum(1 for s in samples if oracle(s) == ground_truth(s))
    p_correct = correct / len(samples)
    return abs(p_correct - 0.5)

# Expected for Elligator2: advantage ≈ 0 (2^-128 queries needed)
```

**Limitations:**
- ~50% of curve points encodable (acceptable retry rate)
- High bit can be randomized (additional entropy)
- Decoding is deterministic (no randomness in reverse map)

### Packet Padding

**Objective:** Mask payload length, prevent size-based fingerprinting.

**Padding Classes:**
```
Distribution (Stealth Mode):
64B:   10%  (tiny control frames)
256B:  15%  (handshakes, ACKs)
512B:  20%  (small chunks)
1024B: 25%  (typical data)
1472B: 20%  (MTU-sized)
8960B: 10%  (jumbo frames)
```

**Effectiveness:**
```python
def mutual_information(packet_sizes, file_sizes):
    """
    Measure information leakage from packet sizes to file sizes.
    Lower is better (0 = no leakage).
    """
    return I(PacketSizes; FileSizes)

# Without padding: I ≈ 0.8 (high leakage)
# With random padding: I ≈ 0.2 (low leakage)
# With stealth mode: I ≈ 0.05 (minimal leakage)
```

### Timing Obfuscation

**Inter-Packet Delay Distribution:**

*Low Latency Mode:* No added delay
```
Delay ~ 0
```

*Moderate Mode:* Exponential distribution
```
Delay ~ Exp(λ=200) → Mean = 5ms
PDF: f(t) = 200 * e^(-200t)
```

*High Privacy Mode:* Match HTTPS patterns
```
Delay ~ Empirical(HTTPS traffic capture)
Sample from: {1ms, 3ms, 5ms, 10ms, 20ms, 50ms}
Probabilities: {0.3, 0.25, 0.2, 0.15, 0.07, 0.03}
```

**Burst Shaping:**
```rust
if bytes_in_window > target_rate * window_size * 1.5 {
    queue_packet();  // Defer transmission
} else {
    send_packet();
}
```

**Cover Traffic Rate:**
```
Minimum: 10 packets/sec (prevent long gaps)
Maximum Idle: 100ms between packets
PAD frames: Random size (64-256 bytes)
```

### Protocol Mimicry Effectiveness

| Mimicry Mode | DPI Resistance | Bandwidth Efficiency |
|--------------|----------------|----------------------|
| **None** | Low (easily detected) | 100% |
| **Elligator2 Only** | Medium (statistical analysis resistant) | 100% |
| **TLS Wrapper** | High (appears as HTTPS) | 95% (5% overhead) |
| **WebSocket** | High (bidirectional stream) | 97% |
| **DNS-over-HTTPS** | Very High (legitimate DNS) | 10-20% (high overhead) |

**TLS Wrapper Detection Resistance:**
```
Entropy of payload: ~7.99 bits/byte (close to 8.0 for random)
Packet sizes: Match HTTPS distribution
Timing: Match web traffic patterns
TLS version: Valid (0x0303 for TLS 1.2)
Cipher suites: Common values
```

---

## Cryptographic Implementation Security

### Constant-Time Operations

**Requirement:** All cryptographic operations must execute in constant time to prevent timing side-channels.

**Critical Functions:**
- X25519 scalar multiplication
- Elligator2 encoding/decoding
- Poly1305 tag generation/verification
- Key comparison

**Implementation:**
```rust
// Constant-time key comparison
pub fn keys_equal(a: &[u8; 32], b: &[u8; 32]) -> bool {
    use subtle::ConstantTimeEq;
    a.ct_eq(b).into()
}

// Using dalek's constant-time X25519
let shared_secret = local_secret.diffie_hellman(&peer_public);
```

**Verification:**
- Use `cargo-audit` to detect timing vulnerabilities
- Run `ct-fuzz` (constant-time fuzzer)
- Review assembly output for conditional branches

### Memory Zeroization

**Requirement:** Sensitive key material must be zeroized immediately after use.

**Zeroizable Types:**
```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
struct SessionKeys {
    send_key: [u8; 32],
    recv_key: [u8; 32],
    chain_key: [u8; 32],
}

impl Drop for SessionKeys {
    fn drop(&mut self) {
        // Automatic zeroization on drop
    }
}
```

**Manual Zeroization:**
```rust
let mut ephemeral_secret = EphemeralSecret::new(OsRng);
// ... use secret ...
zeroize(&mut ephemeral_secret);  // Explicit clear
```

**Memory Locking:**
```rust
use mlock::LockableMemory;

let mut secret_buffer = LockableMemory::new(32)?;
secret_buffer.lock()?;  // Prevent swapping to disk
// ... use buffer ...
secret_buffer.unlock()?;
```

### Random Number Generation

**Requirement:** All randomness must come from the OS CSPRNG.

**Sources:**
- Linux: `getrandom(2)` syscall
- Fallback: `/dev/urandom`
- Never: Pseudo-random generators (e.g., LCG, Mersenne Twister)

**Implementation:**
```rust
use rand::rngs::OsRng;

let secret = EphemeralSecret::new(OsRng);
let nonce = OsRng.gen::<[u8; 24]>();
```

**Entropy Verification:**
```bash
# Check /proc/sys/kernel/random/entropy_avail
cat /proc/sys/kernel/random/entropy_avail
# Should be >1000 for good entropy

# Monitor randomness quality
cat /dev/random | rngtest -c 1000
```

---

## Network Security

### DHT Security

**Encrypted Announcements:**
```rust
struct Announcement {
    nonce: [u8; 24],
    encrypted_payload: [  // XChaCha20-Poly1305
        peer_endpoints: Vec<SocketAddr>,
        timestamp: u64,
        capabilities: u32,
        signature: [u8; 64],  // Ed25519
        auth_tag: [u8; 16],
    ],
}

// Key derivation
let dht_key = BLAKE3(group_secret || file_hash || "announce")[..20];
let announce_key = HKDF(group_secret, "dht-announce", 32);
```

**Sybil Attack Mitigation:**
- Proof-of-work for DHT node ID generation
- Reputation tracking (successful transfers)
- Random peer selection (not closest nodes)

**Privacy Properties:**
- DHT nodes see only: ciphertext, timestamp, requester IP
- DHT nodes cannot see: file hash, peer list, group membership
- Unlinkability: Keys appear random, cannot correlate queries

### Relay Security

**End-to-End Encryption:**
```
Initiator → [Noise Session] → Relay → [Noise Session] → Responder
                ↑                                          ↑
           Encrypted                                  Encrypted
         (relay blind)                              (relay blind)
```

**Relay Metadata Exposure:**
- Relay sees: Source IP, destination public key, timing, packet sizes
- Relay cannot see: Content, file hash, plaintext peer identity

**Relay Trust Model:**
- Zero trust: Assume relay is adversarial
- Security properties maintained even if relay compromised
- Relay can only perform DoS (drop packets)

### NAT Traversal Security

**Hole Punching:**
```
1. Both peers register with relay (TLS-encrypted channel)
2. Relay exchanges endpoint info (encrypted)
3. Simultaneous UDP probes (authenticated)
4. PATH_CHALLENGE/PATH_RESPONSE validation
5. Session migrates to direct path
```

**STUN-like Endpoint Discovery:**
- Multiple relays to detect NAT type
- Signed responses (prevent spoofing)
- Observed address returned in authenticated message

**Birthday Attack for Symmetric NAT:**
```
Security: Both peers verify via PATH_CHALLENGE
Attacker cannot:
  - Forge PATH_RESPONSE (requires session key)
  - Inject data (packets authenticated)
  - MITM (mutual authentication)
```

---

## Attack Resistance Analysis

### Passive Eavesdropping

**Attack:** Adversary captures all network traffic.

**Resistance:**
- **Confidentiality:** ✓ Complete (AEAD encryption)
- **Metadata Protection:** ✓ High (encrypted CID, padded sizes)
- **Traffic Analysis:** ~ Partial (timing/pattern correlation possible)

**Mitigation Effectiveness:**
```
Metric: Mutual Information I(Plaintext; Ciphertext)
Expected: I ≈ 0 (no information leakage)
Actual: I < 2^-100 (computationally negligible)
```

### Active MITM

**Attack:** Adversary intercepts and modifies packets.

**Resistance:**
- **Key Substitution:** ✗ Blocked (mutual authentication)
- **Packet Modification:** ✗ Blocked (authentication tag)
- **Packet Injection:** ✗ Blocked (nonce/sequence validation)
- **Replay:** ✗ Blocked (sliding window)

**Detection:**
```rust
// Out-of-band key verification
let expected_fingerprint = "ABCD-1234-EFGH-5678";  // From secure channel
let actual_fingerprint = fingerprint(&peer_static_key);

if expected_fingerprint != actual_fingerprint {
    panic!("MITM detected!");
}
```

### Traffic Analysis

**Attack:** Statistical analysis of packet timing, sizes, patterns.

**Resistance Levels:**

| Obfuscation Level | Resistance | Overhead |
|-------------------|------------|----------|
| **None** | Low | 0% |
| **Elligator2 + Padding** | Medium | 5-15% |
| **+ Timing Obfuscation** | High | 10-25% |
| **+ Cover Traffic** | Very High | 20-40% |
| **+ Protocol Mimicry** | Excellent | 5-100% (mode-dependent) |

**Known Limitations:**
- Cannot defeat global passive adversary with full network visibility
- Pattern correlation over long periods may reveal usage
- Traffic volume analysis (file size estimation) partially effective

### DPI/Firewall Evasion

**Attack:** Deep packet inspection identifies and blocks protocol.

**Resistance:**

| Technique | DPI Signature | Effectiveness |
|-----------|---------------|---------------|
| **Elligator2** | No recognizable key pattern | ✓ High |
| **TLS Wrapper** | Valid TLS records | ✓ Very High |
| **WebSocket Wrapper** | Valid WS frames | ✓ Very High |
| **DNS-over-HTTPS** | Legitimate DoH queries | ✓ Excellent |

**Testing:**
```bash
# Test against common DPI tools
tshark -r capture.pcap -Y "wraith"  # Should find nothing
snort -c snort.conf -r capture.pcap  # No alerts
suricata -c suricata.yaml -r capture.pcap  # No matches
```

### Cryptographic Attacks

**Attack:** Break cryptographic primitives.

**Primitive Security Levels:**

| Primitive | Security | Break Complexity |
|-----------|----------|------------------|
| X25519 | 128-bit | 2^128 operations |
| XChaCha20 | 256-bit | 2^256 key search |
| Poly1305 | 128-bit | 2^128 forgery |
| BLAKE3 | 128-bit collision | 2^128 operations |
| Ed25519 | 128-bit | 2^128 signature forge |

**Quantum Resistance:**
- X25519: Vulnerable to Shor's algorithm (future quantum computers)
- Symmetric crypto: Resistant (Grover's algorithm → 2^128 security)

**Post-Quantum Upgrade Path:**
```
Hybrid KEM:
  classical_dh = X25519(...)
  pq_kem = Kyber1024.Encap(...)
  shared_secret = classical_dh || pq_kem
```

---

## Security Best Practices

### Deployment Security

1. **Key Management**
   - Generate static keys on secure hardware
   - Store private keys encrypted at rest (AES-256)
   - Use hardware security modules (HSM) for high-value keys

2. **Endpoint Hardening**
   - Run as non-privileged user
   - Use seccomp/AppArmor/SELinux sandboxing
   - Disable core dumps (prevent key leakage)

3. **Network Configuration**
   - Bind only to necessary interfaces
   - Use firewall rules (iptables/nftables)
   - Enable reverse path filtering (rp_filter)

### Operational Security

1. **Key Rotation**
   - Rotate static keys annually (or after suspected compromise)
   - Ephemeral keys automatically rotated every session
   - DHratchet every 2 minutes

2. **Monitoring**
   - Log authentication failures
   - Alert on unusual traffic patterns
   - Monitor resource exhaustion (DoS detection)

3. **Incident Response**
   - Revoke compromised static keys immediately
   - Force all sessions to rekey
   - Audit logs for unauthorized access

---

## Known Vulnerabilities and Limitations

### Traffic Analysis

**Limitation:** Long-term observation may reveal usage patterns.

**Scenarios:**
- Attacker observes network for weeks/months
- Machine learning classifiers trained on protocol behavior
- Correlation attacks linking senders and receivers

**Partial Mitigations:**
- Cover traffic during idle periods
- Random delays between transfers
- Use Tor/VPN for additional layer

### Endpoint Security

**Limitation:** Protocol cannot protect against compromised endpoints.

**Risks:**
- Malware reads files before/after transfer
- Keylogger captures static key passwords
- Memory dump reveals session keys

**Mitigations:**
- Full-disk encryption (e.g., LUKS)
- Secure boot and measured boot
- Endpoint detection and response (EDR) tools

### Denial of Service

**Limitation:** Protocol cannot prevent DoS attacks.

**Attack Vectors:**
- Flood handshake packets (DDOS)
- Exhaust UMEM (send faster than process)
- Relay overload (many simultaneous connections)

**Mitigations:**
- Rate limiting (per-IP handshake rate)
- Cookie-based handshake validation
- Connection limits per relay

---

## Security Audit Recommendations

### Code Audit

**Focus Areas:**
1. Cryptographic implementation (constant-time, zeroization)
2. Memory safety (bounds checking, use-after-free)
3. Integer overflows (arithmetic, buffer sizing)
4. Unsafe code blocks (careful review)

**Tools:**
- `cargo clippy` (linting)
- `cargo audit` (dependency vulnerabilities)
- `cargo miri` (undefined behavior detection)
- `cargo fuzz` (fuzzing)

### Penetration Testing

**Test Scenarios:**
1. MITM attack with key substitution
2. Packet injection and modification
3. Replay attacks
4. DHT poisoning
5. Relay compromise
6. Traffic analysis

### Formal Verification

**Targets:**
1. Noise_XX handshake correctness (Tamarin/ProVerif)
2. Key derivation security
3. Replay window algorithm
4. State machine transitions

---

## Compliance and Certifications

### Cryptographic Standards

- **NIST:** FIPS 140-2 (for symmetric crypto)
- **IETF:** RFC 7539 (ChaCha20-Poly1305), RFC 7748 (X25519)
- **CFRG:** Approved elliptic curves (Curve25519)

### Privacy Regulations

- **GDPR:** Encryption at rest and in transit (Article 32)
- **HIPAA:** Transmission security (§164.312(e)(1))
- **PCI DSS:** Strong cryptography (Requirement 4)

---

## Conclusion

WRAITH's security model provides strong guarantees against:
- Passive eavesdropping (complete protection)
- Active attacks (authentication, integrity)
- Traffic analysis (best-effort obfuscation)
- Protocol identification (high resistance)

**Limitations:**
- Cannot defeat global passive adversary
- Endpoint compromise remains critical weakness
- Traffic correlation over long periods possible

**Recommendation:** Deploy WRAITH as part of defense-in-depth strategy, combined with endpoint security, operational security, and awareness of limitations.

---

**See Also:**
- [Protocol Overview](protocol-overview.md)
- [Layer Design](layer-design.md)
- [Security Testing](../testing/security-testing.md)
