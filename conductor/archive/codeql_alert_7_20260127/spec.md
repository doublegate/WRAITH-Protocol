# Track Specification: Full Remediation for CodeQL Alert #7 (Spectre Implant)

## 1. Overview
This track addresses CodeQL Alert #7: "Cleartext logging of sensitive information" in the WRAITH Spectre implant. The goal is to eliminate cleartext credential material in memory and at rest by implementing a multi-layered defense strategy involving encryption wrappers, secure buffers, and mandatory zeroization.

## 2. Functional Requirements

### 2.1 SensitiveData & SecureBuffer Primitives
*   **SensitiveData<T>:** Implement a wrapper that encrypts data in memory using XChaCha20-Poly1305. 
    *   Uses ephemeral keys generated at instantiation.
    *   Requires an explicit `.unlock()` call which returns a `SensitiveGuard`.
    *   The guard ensures zeroization of plaintext on drop.
*   **SecureBuffer:** Implement a buffer that uses `mlock` (Linux) or `VirtualLock` (Windows) to prevent paging to swap.
*   **Zeroize Integration:** Implement `Zeroize` and `ZeroizeOnDrop` for all structures handling sensitive data.

### 2.2 Component Remediation
*   **Shell Output (shell.rs):** Return `SensitiveData<Vec<u8>>` from `Shell::exec()`. Ensure command strings are zeroized after execution.
*   **LSASS Dumps (credentials.rs):** Modify LSASS dumping to use an in-memory buffer via `MiniDumpWriteDump` callbacks. Encrypt the buffer using XChaCha20-Poly1305 before writing to disk.
*   **Keylogger (collection.rs):** Zeroize the `KEY_BUFFER` immediately after polling and wrap the result in `SensitiveData`.
*   **Discovery & Lateral Movement:** Audit `discovery.rs` and `lateral.rs` to wrap user info and credentials in `SensitiveData`.

### 2.3 Entropy & Crypto Infrastructure
*   **no_std Crypto:** Attempt to use the existing `wraith-crypto` crate with `default-features = false`. Fall back to local `no_std` implementation if `std` dependencies cannot be resolved.
*   **Tiered Entropy:** Implement hardware-based entropy first (`RDRAND`/`RDTSC`), falling back to less secure but functional sources (stack address entropy, counter-based LCG) if hardware instructions are unavailable.

## 3. Non-Functional Requirements
*   **Target:** `no_std` Rust (edition 2021).
*   **Zero Stub Policy:** All encryption and locking logic must be real and functional. No placeholders.
*   **Performance:** Memory locking and encryption should be targeted to avoid system-wide performance degradation.

## 4. Acceptance Criteria
*   CodeQL Alert #7 is marked as RESOLVED in scanning.
*   `SensitiveData` unit tests verify encryption/decryption round-trips.
*   `SecureBuffer` unit tests verify memory locking success (where privileges allow).
*   LSASS dumps on disk are verified to be encrypted (no cleartext "MDMP" header).
*   Memory audit confirms zeroization of intermediate plaintext buffers.
