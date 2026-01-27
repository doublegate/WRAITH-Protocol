# WRAITH Protocol v2 Cryptographic Claims: Validation Report

The WRAITH Protocol v2 cryptographic specification contains **several inaccurate claims** that require correction, alongside constructions that are fundamentally sound but lack formal standardization. Two findings are critical: the stated security levels for ML-KEM-768 and ML-DSA-65 are **incorrect by an entire NIST category**, and the hybrid signature binding mechanism is **incomplete for preventing key-mismatch attacks**.

## Security level claims misstate NIST classifications by one category

The document's most significant error lies in its security level claims. Both **ML-KEM-768 and ML-DSA-65 are NIST Security Category 3** (approximately 192-bit equivalent), not Category 1 (128-bit equivalent) as stated.

Per FIPS 203 and FIPS 204, published August 13, 2024:

| Algorithm | Document Claims | Actual NIST Category | Equivalent Strength |
|-----------|-----------------|---------------------|---------------------|
| ML-KEM-768 | Category 1, 128-bit PQ | **Category 3** | ~AES-192 |
| ML-DSA-65 | 128-bit PQ | **Category 3** | ~AES-192 |

The attack complexity figures are also **inconsistent** with Category 3 security. The claimed "~2^146 classical operations" for ML-KEM-768 corresponds to NIST Level 2 (SHA-256 collision), while Category 3 requires resistance to ~**2^207 classical gate operations** (AES-192 key search equivalent). The ML-DSA-65 claim of "~2^150 operations" similarly falls short of Category 3 thresholds. Notably, these misstatements actually **underrepresent** the algorithms' security—both provide stronger protection than documented.

Recent cryptanalysis (2024-2025) has not fundamentally weakened either algorithm. Research has focused on implementation-level concerns such as side-channel attacks and fault injection, rather than algorithmic breaks. NIST selected HQC as a backup KEM in March 2025 specifically to provide defense-in-depth through mathematical diversity, not due to concerns about ML-KEM.

## The hybrid KEM construction is provably secure under standard assumptions

The claim that "the combined secret is secure if EITHER X25519 OR ML-KEM is secure" is **provably true** under well-established cryptographic assumptions. The foundational proofs from Bindel, Brendel, Fischlin, and Stebila (2019), along with the X-Wing paper by Barbosa et al. (2024), establish that hybrid KEMs using proper combiners achieve this "secure if either" guarantee.

The security relies on the combiner function satisfying either the **split-key PRF** or **dual PRF** assumption. For a function dPRF(k₁, k₂), this means it must behave as a pseudorandom function when either key input is random (with the other potentially adversarially controlled). BLAKE3's keyed hash mode is designed and widely accepted as a PRF; however, **no published formal proof confirms BLAKE3 as a dual PRF**. The IETF draft on KEM combiners explicitly notes this gap, stating that such constructions are assumed secure "in practice, despite not having formal security proofs."

For IND-CCA2 preservation, the construction requires including sufficient binding context in the combiner. The X-Wing specification demonstrates that ML-KEM-768 satisfies the **C2PRI (Ciphertext Second-Preimage Resistance)** property due to its Fujisaki-Okamoto transform, allowing optimization of the combiner. However, X25519 does not satisfy C2PRI when viewed as a KEM, meaning the combiner **must include** the X25519 ciphertext and public key:

```
ss = BLAKE3_keyed(ss_ML-KEM || ss_X25519 || ct_X25519 || pk_X25519 || label)
```

No known cryptographic weaknesses exist in combining X25519 with ML-KEM-768. This pairing is increasingly standard—X-Wing has an active IETF draft (draft-connolly-cfrg-xwing-kem-09) and is designed specifically for this combination.

## HKDF-BLAKE3 is functional but unstandardized

BLAKE3 **satisfies the PRF requirements** necessary for HKDF's underlying hash function. Its keyed mode is explicitly designed as a PRF/MAC replacement, and the compression function inherits extensive analysis from the BLAKE/BLAKE2/ChaCha lineage. However, "HKDF-BLAKE3" as a literal construction (using BLAKE3 within RFC 5869's framework) has **not been formally analyzed or standardized**.

BLAKE3's authors recommend using its native `derive_key` mode rather than wrapping it in HKDF. This native KDF uses a two-phase approach with context string hashing followed by key material derivation, providing equivalent functionality with cleaner security arguments. The key distinction:

- **BLAKE3 derive_key**: Uses `context_string + key_material` (recommended)
- **HKDF**: Uses `salt + IKM + info` (different API semantics)

Current standardization status remains preliminary. The IETF draft (draft-aumasson-blake3-00) from July 2024 is not yet an RFC, and BLAKE3 lacks NIST approval for FIPS contexts. For applications requiring proven, standardized constructions, HKDF-SHA256 or HKDF-SHA384 remain the conservative choices. For performance-critical applications where BLAKE3's ~3 GB/s throughput matters, the construction is cryptographically sound in practice.

## The forward secrecy ratchet works but lacks post-compromise security

The hash-based ratchet using BLAKE3 keyed hash **is sound for forward secrecy**. BLAKE3's keyed mode satisfies the core requirements: PRF security ensures output indistinguishability, and the one-way property prevents backward computation. Deleting `chain_key[n-1]` after deriving `chain_key[n]` is exactly the correct mechanism—this matches the Signal Protocol's symmetric ratchet approach, proven secure in the EUROCRYPT 2019 analysis by Alwen, Coretti, and Dodis.

The construction's **critical limitation** is that it provides forward secrecy only, not **post-compromise security** (break-in recovery). If an attacker compromises `chain_key[n]`, they can compute all future chain keys: `chain_key[n+1]`, `chain_key[n+2]`, etc. Signal addresses this through its Double Ratchet, which combines:
- Symmetric ratchet (KDF chain) → Forward secrecy
- DH ratchet → Post-compromise security

For per-packet encryption where sessions are short-lived and post-compromise recovery is less critical, this construction suffices. For long-lived secure channels, combining this ratchet with periodic DH or KEM exchanges would provide stronger guarantees. Recent research (IACR 2024/220) also recommends strict limits on chain length for very long sessions due to theoretical collision concerns.

## Hybrid signature binding is incomplete and may permit key-mismatch attacks

Having Ed25519 sign the ML-DSA-65 public key **alone is insufficient** for robust key-mismatch attack prevention. This construction has several gaps identified against the IETF draft-ietf-lamps-pq-composite-sigs framework:

- **No bidirectional binding**: Ed25519 binds to ML-DSA-65, but not vice versa. An attacker could create their own Ed25519 key and sign a victim's ML-DSA-65 public key.
- **No identity binding**: Neither signature ties to an identity string or application context.
- **No domain separator**: Without algorithm-specific domain separation, cross-protocol attacks remain possible.
- **Weak non-separability**: Simply signing one key with another doesn't achieve the weak non-separability property defined in IETF hybrid signature specifications.

The IETF composite signature draft defines `id-MLDSA65-Ed25519` as a proper composite algorithm. Correct binding requires both algorithms to sign a modified message incorporating:

```
M' = Domain_Separator || len(context) || context || message
```

Where the message being signed should include **both public keys** and application-specific context. This ensures that signatures cannot be separated, replayed, or misbounded across protocols.

## Nonce construction is safe under implementation constraints

The session-derived mask XORed with counter construction for XChaCha20-Poly1305 is **conditionally safe**. This approach closely mirrors TLS 1.3's nonce generation (IV XOR sequence_number) and is a well-established pattern.

XChaCha20-Poly1305's **192-bit nonce** provides substantial margin—50% collision probability requires ~2^96 messages under a single key. The mask-XOR-counter approach guarantees uniqueness within a session because if `counter_i ≠ counter_j`, then `(mask ⊕ counter_i) ≠ (mask ⊕ counter_j)`. Safety depends on:

- Session mask derived cryptographically via HKDF or similar from session key material
- Counter never resets within a session lifetime
- Counter overflow forces session re-keying
- Different sessions produce different masks

This construction is **not nonce-misuse resistant**. Unlike AES-GCM-SIV, XChaCha20-Poly1305 suffers catastrophic failure if the same (key, nonce) pair encrypts two distinct messages. Implementation must absolutely guarantee uniqueness—there is no graceful degradation.

## Specific answers to validation questions

**1. Is the "secure if either is secure" claim for hybrid KEMs provably true?**
Yes. Under the dual PRF assumption for the combiner function, hybrid KEMs preserve IND-CCA2 security if either component KEM is secure. This is proven in multiple academic papers (Bindel et al. 2019, Giacon et al. 2018, X-Wing 2024). BLAKE3 keyed hash is assumed to satisfy this in practice but lacks formal proof.

**2. What are the actual NIST security levels for ML-KEM-768 and ML-DSA-65?**
Both are **NIST Security Category 3** (~AES-192 equivalent, ~192-bit security), not Category 1 as claimed. This requires resistance to ~2^207 classical gate operations.

**3. Is HKDF-BLAKE3 a recognized construction with security proofs?**
No. BLAKE3's native `derive_key` mode is recommended by its authors as an HKDF replacement, but "HKDF-BLAKE3" specifically (BLAKE3 within RFC 5869's framework) is not standardized or formally analyzed. BLAKE3 satisfies PRF requirements in principle.

**4. Are there any known weaknesses in combining X25519 with ML-KEM this way?**
No cryptographic weaknesses are known. The combination is increasingly standard (X-Wing draft). Implementation must include X25519 ciphertext in the combiner hash since X25519 does not satisfy C2PRI.

**5. Does the proof sketch in the document hold under standard cryptographic assumptions?**
Partially. The hybrid KEM security relies on BLAKE3 being a dual PRF (assumed but unproven). The forward secrecy ratchet is sound. The hybrid signature binding requires strengthening. The security level claims require correction from Category 1 to Category 3.

## Conclusion

The WRAITH Protocol v2 demonstrates sound cryptographic architecture for a post-quantum hybrid system, but requires corrections before deployment. The **most critical issues** are the misstated security levels (which actually understate the algorithms' strength) and the incomplete hybrid signature binding. The hybrid KEM construction, BLAKE3-based KDF, and forward secrecy ratchet are **cryptographically sound in practice** but rely on assumptions without formal proofs in some cases. For production deployment, the protocol should: correct security level documentation to reflect NIST Category 3; enhance signature binding per IETF composite signature drafts; and consider whether the lack of formal BLAKE3 dual-PRF proofs is acceptable for the target threat model.