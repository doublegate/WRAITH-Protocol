# CodeQL Alert #7: Cleartext Logging of Sensitive Information

## Alert Metadata

| Field | Value |
|-------|-------|
| **Alert Number** | #7 |
| **Severity** | HIGH |
| **CWE IDs** | CWE-312 (Cleartext Storage of Sensitive Information), CWE-359 (Exposure of Private Personal Information), CWE-532 (Insertion of Sensitive Information into Log File) |
| **CodeQL Rule** | `rust/cleartext-logging` |
| **File** | `clients/wraith-redops/spectre-implant/src/modules/shell.rs` |
| **Line** | 75 (Unix path), columns 33-42 |
| **CodeQL Message** | "This operation writes username to a log file. This operation writes password to a log file." |
| **Tool** | CodeQL Security Scan (GitHub Code Scanning) |
| **Priority** | P1 |
| **Status** | Open |
| **Date Filed** | 2026-01-27 |

---

## Problem Description

CodeQL's `rust/cleartext-logging` rule detected that the Spectre implant's shell execution module handles potentially sensitive data (usernames, passwords, credentials) in cleartext. Specifically:

1. The `Shell::exec()` method at `shell.rs:57` accepts a `cmd: &str` parameter that may contain credentials (e.g., commands like `net user admin password123`, `runas /user:DOMAIN\admin`, `smbclient -U user%pass`).

2. At line 75 (`let mut cmd_c = Vec::from(cmd.as_bytes());`), the command string -- which may embed credentials -- is converted to raw bytes and passed directly to the OS shell (`/bin/sh -c` on Unix, `cmd.exe /c` on Windows) without any sanitization or encryption.

3. The command output (captured into `output: Vec<u8>`) may contain credential data echoed back by the executed program, and this output is returned as raw cleartext bytes to the caller.

This is a valid security concern for a red team implant: credential material flowing through shell command execution should be treated as sensitive and protected in-memory and in-transit.

---

## Analysis

### Data Flow

```
Operator Command (may contain credentials)
    |
    v
Shell::exec(cmd: &str)           <-- cmd may embed usernames/passwords
    |
    v
Vec::from(cmd.as_bytes())        <-- Line 75: cleartext byte conversion (FLAGGED)
    |
    v
sys_execve / CreateProcessA      <-- OS shell runs the command
    |
    v
output: Vec<u8>                  <-- Command stdout/stderr captured in cleartext
    |
    v
Returned to caller               <-- Cleartext output sent back to operator
```

### Related Sensitive Data Paths

The `credentials.rs` module in the same crate performs LSASS memory dumps:

- `Credentials::dump_lsass(target_path: &str)` opens the LSASS process with `PROCESS_ALL_ACCESS`, calls `MiniDumpWriteDump`, and writes the full memory dump to `target_path` **in cleartext** on disk.
- The LSASS dump file contains Windows credential material (NTLM hashes, Kerberos tickets, plaintext passwords) and is stored as an unencrypted file at the operator-specified path.

### Additional Modules with Potential Credential Exposure

| Module | Concern |
|--------|---------|
| `shell.rs` | Command strings and output may contain credentials |
| `credentials.rs` | LSASS dump written to disk in cleartext |
| `powershell.rs` | PowerShell commands may contain credentials in script blocks |
| `lateral.rs` | Lateral movement may pass credential material |
| `collection.rs` | Data collection may capture credential stores |
| `discovery.rs` | Network discovery output may include service account info |

---

## Remediation Plan

### Step 1: Create `SensitiveData` Wrapper Type

Create a `SensitiveData<T>` wrapper type that:
- Encrypts the inner value on creation using XChaCha20-Poly1305 from `wraith-crypto`
- Only decrypts with an explicit `.unlock()` call that returns a guard
- The guard auto-re-encrypts on drop
- Implements `Zeroize` to clear the encryption key material from memory

```rust
// Conceptual API
pub struct SensitiveData<T: AsRef<[u8]>> {
    encrypted: Vec<u8>,
    nonce: [u8; 24],
    key: zeroize::Zeroizing<[u8; 32]>,
}

impl<T: AsRef<[u8]>> SensitiveData<T> {
    pub fn new(plaintext: T) -> Self { /* encrypt with ephemeral key */ }
    pub fn unlock(&self) -> SensitiveGuard<T> { /* decrypt, return guard */ }
}

impl<T: AsRef<[u8]>> Drop for SensitiveData<T> {
    fn drop(&mut self) { /* zeroize all fields */ }
}
```

### Step 2: Apply `SensitiveData` to Shell Output

Modify `Shell::exec()` to return `SensitiveData<Vec<u8>>` instead of raw `Vec<u8>`:

- Wrap the command output buffer in `SensitiveData` before returning
- Callers must explicitly `.unlock()` to access the output
- The raw output buffer is zeroized immediately after encryption

### Step 3: Encrypt LSASS Dump at Rest

Modify `Credentials::dump_lsass()` to:

- Write the MiniDump to a temporary in-memory buffer (or encrypt in streaming fashion)
- Encrypt the dump data using XChaCha20-Poly1305 before writing to `target_path`
- Prepend a header with the encrypted key (wrapped with the operator's session key) so the team server can decrypt
- Alternatively, stream-encrypt chunks as they are written to avoid holding the full dump in memory

### Step 4: Add `Zeroize` Implementations

Add `zeroize` crate dependency and implement `Zeroize` + `ZeroizeOnDrop` for:

- All `Vec<u8>` buffers that hold command output in `shell.rs`
- The command string buffer (`cmd_c`) in `shell.rs`
- Process information structures in `credentials.rs`
- Any intermediate buffers in `powershell.rs` and `lateral.rs`

### Step 5: Create `SecureBuffer` Type

Create a `SecureBuffer` type that:
- Allocates memory with `mlock()` (Unix) / `VirtualLock()` (Windows) to prevent paging to swap
- Auto-zeroizes on drop via the `Zeroize` trait
- Provides `AsRef<[u8]>` and `AsMut<[u8]>` for seamless use

### Step 6: Audit All Modules for Credential Data Paths

Review every module in `spectre-implant/src/modules/` to identify:
- Functions that accept or return potentially sensitive data
- Buffers that hold credential material even transiently
- File I/O paths that write sensitive data to disk
- Network I/O paths that transmit sensitive data

---

## Affected Files

| File | Path | Concern |
|------|------|---------|
| `shell.rs` | `clients/wraith-redops/spectre-implant/src/modules/shell.rs` | **PRIMARY**: Cleartext command strings and output (line 75) |
| `credentials.rs` | `clients/wraith-redops/spectre-implant/src/modules/credentials.rs` | Cleartext LSASS dump written to disk at `target_path` |
| `powershell.rs` | `clients/wraith-redops/spectre-implant/src/modules/powershell.rs` | PowerShell command execution may contain credentials |
| `lateral.rs` | `clients/wraith-redops/spectre-implant/src/modules/lateral.rs` | Lateral movement credential handling |
| `collection.rs` | `clients/wraith-redops/spectre-implant/src/modules/collection.rs` | Data collection may gather credential stores |
| `discovery.rs` | `clients/wraith-redops/spectre-implant/src/modules/discovery.rs` | Network discovery may expose service credentials |
| `mod.rs` | `clients/wraith-redops/spectre-implant/src/modules/mod.rs` | Module dispatch may route credential data |

---

## Acceptance Criteria

- [ ] `SensitiveData<T>` wrapper type implemented with XChaCha20-Poly1305 encryption from `wraith-crypto`
- [ ] `Shell::exec()` returns encrypted output; callers use `.unlock()` to access
- [ ] `Credentials::dump_lsass()` encrypts the LSASS dump file at rest before writing to disk
- [ ] `Zeroize` and `ZeroizeOnDrop` applied to all credential-bearing buffers
- [ ] `SecureBuffer` type implemented with memory locking (`mlock`/`VirtualLock`)
- [ ] All modules in `spectre-implant/src/modules/` audited for cleartext credential paths
- [ ] CodeQL re-scan confirms Alert #7 is resolved (no new findings introduced)
- [ ] Unit tests verify encryption/decryption round-trip for `SensitiveData`
- [ ] Unit tests verify zeroization occurs on drop (check memory contents)
- [ ] No regression in existing test suite (2,140+ tests passing)

---

## References

- [CWE-312: Cleartext Storage of Sensitive Information](https://cwe.mitre.org/data/definitions/312.html)
- [CWE-359: Exposure of Private Personal Information to an Unauthorized Actor](https://cwe.mitre.org/data/definitions/359.html)
- [CWE-532: Insertion of Sensitive Information into Log File](https://cwe.mitre.org/data/definitions/532.html)
- [OWASP Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)
- [CodeQL rust/cleartext-logging rule documentation](https://codeql.github.com/codeql-query-help/rust/rust-cleartext-logging/)
- [zeroize crate (RustCrypto)](https://docs.rs/zeroize/latest/zeroize/)
- [WRAITH Protocol XChaCha20-Poly1305 AEAD](../../crates/wraith-crypto/src/aead.rs)
- [GitHub CodeQL Action Issue #3413](https://github.com/github/codeql-action/issues/3413) (platform context)
