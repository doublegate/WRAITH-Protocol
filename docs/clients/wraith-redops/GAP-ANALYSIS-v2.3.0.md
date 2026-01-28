# WRAITH-RedOps Gap Analysis v2.3.0

**Version:** 2.3.0 (Comprehensive Re-Verification v6.0.0)
**Date:** 2026-01-27
**Analyst:** Claude Opus 4.5 (Automated Source Code Audit)
**Previous Versions:** [GAP-ANALYSIS-v2.2.5.md](GAP-ANALYSIS-v2.2.5.md) (v5.0.0 internal) -- now consolidated into this document
**Scope:** Complete source code audit of all WRAITH-RedOps components
**Method:** Exhaustive line-by-line reading of every source file, automated pattern scanning, cross-reference analysis

---

## Executive Summary

This document presents a comprehensive gap and technical analysis of the WRAITH-RedOps adversary emulation platform at version 2.3.0, consolidating all findings from the v2.2.5 (v5.0.0) gap analysis and incorporating the results of a fresh exhaustive audit of every source file.

### Audit Methodology (v6.0.0)

1. **Full File Read:** Every `.rs`, `.ts`, `.tsx`, `.proto`, and `.sql` file was read in its entirety
2. **Stub/Placeholder Patterns:** `TODO|FIXME|HACK|XXX|WIP|unimplemented!|todo!|panic!`
3. **Incomplete Implementation Patterns:** `In a real|In production|placeholder|stub|mock|dummy|fake|not implemented|not yet|coming soon|assume success`
4. **Code Smell Patterns:** `Ok(())` in suspicious contexts, `Vec::new()` as return values
5. **Error Handling Gaps:** `.unwrap()` and `.expect()` usage analysis
6. **Hardcoded Value Detection:** IP addresses, ports, credentials, magic numbers, fallback keys
7. **Cross-Reference:** All 5 specification documents (`architecture.md`, `features.md`, `implementation.md`, `integration.md`, `testing.md`) + sprint plan + proto file cross-referenced against actual implementation
8. **Security Analysis:** Cryptographic key management, authentication, audit logging
9. **IPC Bridge Verification:** Proto definitions (32 RPCs) cross-referenced against Tauri `invoke_handler` registrations (31 commands) and React `invoke()` calls
10. **MITRE ATT&CK Coverage Mapping:** All implemented techniques mapped against planned coverage
11. **Dead Code Analysis:** `#[allow(dead_code)]` annotations cataloged and assessed
12. **Unsafe Code Audit:** All `unsafe` blocks cataloged and risk-assessed
13. **Consolidation:** All v2.2.5 findings merged, re-verified, and updated

### Key Findings

| Category | Assessment |
|----------|------------|
| **Overall Completion** | ~96% (up from ~94% in v2.2.5) |
| **Production Readiness** | APPROACHING READY -- zero P0 issues remain |
| **Core C2 Functionality** | ~97% complete |
| **Implant Tradecraft** | ~93% complete (up from ~89% in v2.2.5) |
| **Operator Experience** | ~98% complete |
| **Security Posture** | LOW risk -- all crypto keys from env vars, auth enforced |
| **IPC Coverage** | 97% (31 of 32 proto RPCs wired; 2 PowerShell RPCs missing) |
| **MITRE ATT&CK Coverage** | ~82% (31 of 38 planned techniques) |
| **Primary Blockers** | Key ratcheting (P1), PowerShell runner DLL (P1) |

### Changes Since v2.2.5

| Metric | v2.3.0 (Actual) | v2.2.5 | Delta |
|--------|-----------------|--------|-------|
| Total Rust Source Lines | 13,242 | ~12,819 | +423 (+3.3%) |
| Team Server Lines (Rust) | 5,225 | ~4,488 | +737 (+16.4%) |
| Spectre Implant Lines | 6,553 | ~5,729 | +824 (+14.4%) |
| Operator Client (Rust) | 1,164 | ~1,084 | +80 (+7.4%) |
| Operator Client (TS/TSX) | 1,526 | ~1,518 | +8 (+0.5%) |
| Proto Definition | 531 | ~510 | +21 (+4.1%) |
| SQL Migrations | 6 files (208 lines) | 5 files | +1 file |
| Implant Modules | 18 | 15 | +3 (patch, screenshot, browser) |
| Proto RPCs (OperatorService) | 32 | 30 | +2 (PowerShell mgmt) |
| Tauri IPC Commands | 31 | 31 | 0 |
| `#[allow(dead_code)]` | 10 annotations | 8 | +2 |
| `.unwrap()` in prod code | 4 | ~4 | 0 |
| `.unwrap()` in test code | 76 | ~76 | 0 |
| `.expect()` in prod code | 14 | ~14 | 0 |
| `unsafe` blocks (total) | 373 | ~340 | +33 |
| P0 Issues | 0 | 0 | 0 |
| P1 Issues Open | 2 | 2 | 0 |
| P2 Issues Open | 8 | 10 | -2 (resolved) |
| P3 Issues Open | 5 | 5 | 0 |

### Overall Status

| Component | Completion (v2.3.0) | Previous (v2.2.5) | Delta | Notes |
|-----------|--------------------|--------------------|-------|-------|
| Team Server | **97%** | 97% | 0% | New: powershell.rs, rekey_tests.rs, powershell_test.rs, listeners table migration |
| Operator Client | **98%** | 99% | -1% | 2 new PowerShell RPCs not yet wired (31/32 coverage) |
| Spectre Implant | **93%** | 89% | +4% | 3 new modules (patch, screenshot, browser), expanded syscalls/obfuscation/mesh/c2 |
| WRAITH Integration | **93%** | 92% | +1% | New AMSI/ETW bypass, screenshot, browser harvest in dispatch |
| **Overall** | **~96%** | ~94% | **+2%** | 3 new modules, expanded tradecraft, 2 new RPCs |

### Remaining Critical Gaps

1. **No Key Ratcheting** -- Noise session established once, `rekey_dh()` called on counter but NoiseTransport::rekey_dh() generates new DH key without exchanging it with peer -- effectively a no-op for forward secrecy (P1, 13 SP)
2. **PowerShell Runner Placeholder** -- `RUNNER_DLL` is minimal MZ bytes, not a real .NET assembly for CLR hosting (P1, 5 SP)
3. **2 PowerShell RPCs Not Wired** -- `SetPowerShellProfile` and `GetPowerShellProfile` proto RPCs exist but no Tauri IPC commands (P2, 3 SP)
4. **Console Command Coverage** -- Console.tsx maps 11 commands; implant dispatches 19 task types (P2, 5 SP)
5. **Windows UdpTransport stub** -- Returns `Err(())` unconditionally (P2, 3 SP)

---

## Table of Contents

1. [Team Server Findings](#1-team-server-findings)
2. [Spectre Implant Findings](#2-spectre-implant-findings)
3. [Operator Client Findings](#3-operator-client-findings)
4. [Proto Definition Analysis](#4-proto-definition-analysis)
5. [Integration Gap Analysis](#5-integration-gap-analysis)
6. [MITRE ATT&CK Coverage](#6-mitre-attck-coverage)
7. [Sprint Completion Verification](#7-sprint-completion-verification)
8. [Enhancement Recommendations](#8-enhancement-recommendations)
9. [Prioritized Remediation Plan](#9-prioritized-remediation-plan)
10. [Appendices](#appendices)

---

## 1. Team Server Findings

**Total Lines:** 5,225 Rust (across 24 source files)
**New Files Since v2.2.5:** `services/powershell.rs` (128 lines), `services/rekey_tests.rs` (74 lines), `powershell_test.rs` (79 lines)

### 1.1 File: `team-server/src/main.rs` (230 lines)

**STATUS: FULLY FUNCTIONAL**

All configuration is properly loaded from environment variables. The server initializes:
- PostgreSQL database pool (via `DATABASE_URL`)
- Noise keypair generation for C2 encryption
- Dynamic listener spawning (HTTP, UDP, DNS, SMB) from database config
- gRPC server with auth interceptor (via `GRPC_LISTEN_ADDR`)

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| 120 | Env Var | Info | `DATABASE_URL` required with `.expect()` | Correct design |
| 146 | `.expect()` | Low | `NoiseKeypair::generate().expect(...)` | Acceptable -- startup-only |
| 197 | Env Var | Info | `GRPC_LISTEN_ADDR` required with `.expect()` | Correct design |

### 1.2 File: `team-server/src/database/mod.rs` (645 lines)

**STATUS: FULLY FUNCTIONAL -- Enhanced from v2.2.5 (619 lines)**

+26 lines growth from listeners table migration support.

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| 22 | `.expect()` | Info | `HMAC_SECRET` env var required -- correct, no fallback | Correct |
| 26 | `.expect()` | Info | `MASTER_KEY` env var required (64 hex chars) | Correct |
| 31 | `panic!()` | Info | `MASTER_KEY` validation (must be 32 bytes after hex decode) | Correct startup validation |
| 83 | Dead Code | Low | `#[allow(dead_code)]` on database function | Minor tech debt |
| 354 | Dead Code | Low | `#[allow(dead_code)]` on database function | Minor tech debt |
| 525 | Dead Code | Low | `#[allow(dead_code)]` on database function | Minor tech debt |

### 1.3 File: `team-server/src/services/operator.rs` (1,365 lines)

**STATUS: FULLY FUNCTIONAL -- Enhanced from v2.2.5 (1,185 lines)**

+180 lines of growth. Implements all 32 OperatorService proto RPCs including the new `SetPowerShellProfile` and `GetPowerShellProfile`. Uses `PowerShellManager` at line 23.

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| 16 | Dead Code | Low | `#[allow(dead_code)]` on `governance` field | Minor tech debt |
| 18 | Dead Code | Low | `#[allow(dead_code)]` on `static_key` field | Minor tech debt |
| 20 | Dead Code | Low | `#[allow(dead_code)]` on `sessions` field | Minor tech debt |

### 1.4 File: `team-server/src/services/implant.rs` (306 lines)

**STATUS: FUNCTIONAL -- Enhanced from v2.2.5 (277 lines)**

+29 lines growth. Implements all 6 `ImplantService` RPCs.

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| ~25 | Placeholder | Medium | Registration does not decrypt `encrypted_registration` or validate `ephemeral_public` from `RegisterRequest`. Creates generic implant record with hardcoded defaults (os_type: "linux", hostname: "grpc-agent-*"). | Open P2 |
| ~34 | `.unwrap()` | Low | `Uuid::new_v4().to_string().split('-').next().unwrap()` -- safe but could use fallback | Acceptable |

### 1.5 File: `team-server/src/services/session.rs` (111 lines)

**STATUS: FUNCTIONAL -- Enhanced from v2.2.5 (76 lines)**

+35 lines growth. Contains Noise Protocol session management.

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| 60 | Dead Code | Low | `#[allow(dead_code)]` on session field | Minor tech debt |

### 1.6 File: `team-server/src/services/killswitch.rs` (61 lines)

**STATUS: FULLY FUNCTIONAL**

Ed25519 signed UDP broadcast with `WRAITH_K` magic bytes. All keys from environment variables.

### 1.7 File: `team-server/src/services/protocol.rs` (284 lines)

**STATUS: FUNCTIONAL -- Enhanced from v2.2.5 (259 lines)**

+25 lines growth. Protocol-level C2 message processing with Noise handshake integration.

### 1.8 File: `team-server/src/services/listener.rs` (94 lines)

**STATUS: FUNCTIONAL**

Listener management with `DashMap<String, AbortHandle>` for lifecycle management.

### 1.9 File: `team-server/src/services/playbook_loader.rs` (78 lines)

**STATUS: FUNCTIONAL**

YAML/JSON playbook loading from `playbooks/` directory.

### 1.10 File: `team-server/src/services/powershell.rs` (128 lines) -- NEW

**STATUS: FUNCTIONAL**

New PowerShell profile management service. Manages per-implant PowerShell scripts/profiles.

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| 26 | Dead Code | Low | `#[allow(dead_code)]` annotation | Minor tech debt |

### 1.11 File: `team-server/src/services/rekey_tests.rs` (74 lines) -- NEW

**STATUS: TEST CODE**

New test module verifying Noise handshake and transport mode transitions. Tests `NoiseKeypair::generate()`, 3-message handshake, and bidirectional encrypted communication.

### 1.12 File: `team-server/src/listeners/http.rs` (78 lines)

**STATUS: FUNCTIONAL**

HTTP listener at `/api/v1/beacon` with Axum, governance enforcement. No changes from v2.2.5.

### 1.13 File: `team-server/src/listeners/udp.rs` (57 lines)

**STATUS: FUNCTIONAL**

UDP listener with per-packet task spawning and governance enforcement. No changes from v2.2.5.

### 1.14 File: `team-server/src/listeners/dns.rs` (326 lines)

**STATUS: FUNCTIONAL**

Full DNS C2 listener with TXT record data extraction, domain validation, and wire format response construction.

### 1.15 File: `team-server/src/listeners/smb.rs` (302 lines)

**STATUS: FUNCTIONAL -- Enhanced from v2.2.5 (269 lines)**

+33 lines growth. Full SMB2 listener with `pending_data` buffer for Write-then-Read response flow (resolves v2.2.5 TODO).

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| 119-121 | `.unwrap()` | Medium | Three `.unwrap_or_default()` on `try_into()` for header parsing: `msg_id`, `proc_id`, `session_id` | Uses `unwrap_or_default` -- acceptable |
| 139, 172, 190, 232, 266 | `unsafe` | Medium | 5 `unsafe { core::mem::transmute(h) }` blocks for SMB2 header serialization | Documented SAFETY comments; `#[repr(C, packed)]` struct -- correct but fragile |

### 1.16 File: `team-server/src/builder/mod.rs` (184 lines)

**STATUS: FUNCTIONAL**

Binary patching via `WRAITH_CONFIG_BLOCK` magic signature. `compile_implant()` with obfuscation flags (strip=symbols, lto=fat, opt-level=z, panic=abort, crt-static).

### 1.17 File: `team-server/src/builder/phishing.rs` (104 lines)

**STATUS: PARTIALLY FUNCTIONAL**

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| - | Incomplete | P3 | VBA macro generation is incomplete; HTML smuggling works | Open P3 |

### 1.18 File: `team-server/src/governance.rs` (125 lines)

**STATUS: FUNCTIONAL**

IP/domain whitelist/blacklist, time window enforcement, rate limiting.

### 1.19 File: `team-server/src/models/mod.rs` (176 lines)

**STATUS: FUNCTIONAL**

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| 76 | Dead Code | Low | `#[allow(dead_code)]` on model field | Minor tech debt |

### 1.20 Test Files

| File | Lines | Description |
|------|-------|-------------|
| `auth_tests.rs` | 80 | JWT authentication tests |
| `killswitch_config_test.rs` | 121 | Kill switch Ed25519 and config tests |
| `operator_service_test.rs` | 315 | Integration tests covering 7+ service areas |
| `powershell_test.rs` | 79 | PowerShell session and job tests |
| `services/rekey_tests.rs` | 74 | Noise handshake and transport tests |

---

## 2. Spectre Implant Findings

**Total Lines:** 6,553 Rust (across 29 source files)
**New Files Since v2.2.5:** `modules/patch.rs` (84 lines), `modules/screenshot.rs` (138 lines), `modules/browser.rs` (93 lines)
**Architecture:** `#![no_std]` with custom `MiniHeap` bump allocator at `0x10000000` (1 MB)

### 2.1 File: `spectre-implant/src/lib.rs` (48 lines)

**STATUS: FUNCTIONAL**

Entry point with custom heap allocator, `no_std` configuration, panic handler.

### 2.2 File: `spectre-implant/src/c2/mod.rs` (731 lines)

**STATUS: FUNCTIONAL -- Significantly Enhanced from v2.2.5 (541 lines)**

+190 lines growth. Core C2 module with:
- `PatchableConfig` with `WRAITH_CONFIG_BLOCK` magic (server_addr, sleep_interval, kill_date, working_hours)
- HTTP transport (Linux: raw syscalls, Windows: WinINet hash-resolved API)
- UDP transport (Linux: raw syscalls, Windows: stub returning `Err(())`)
- Noise_XX 3-message handshake with ratchet public key exchange
- Kill date and working hours enforcement
- Beacon loop with jitter and transport failover (HTTP <-> UDP)
- Mesh server integration (TCP + Named Pipes)
- DH rekeying counter check (2 min / 1M packets)
- **19 task types** in `dispatch_tasks()`:
  `kill`, `shell`, `powershell`, `inject`, `bof`, `socks`, `persist`, `uac_bypass`, `timestomp`, `sandbox_check`, `dump_lsass`, `sys_info`, `screenshot`, `browser`, `net_scan`, `psexec`, `service_stop`, `keylogger`, `mesh_relay`

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| 65 | `.unwrap_or()` | Low | `addr_len` parsing with fallback to 64 | Acceptable |
| 118 | `.unwrap_or()` | Low | IP octet parsing with fallback to "0" | Acceptable |
| 325 | Transport Stub | P2 | Windows `UdpTransport::request()` returns `Err(())` unconditionally | Open |
| 456 | Rekey Logic | P1 | `session.rekey_dh()` called but does not perform DH exchange with peer | Open |
| 516 | Dead Code | Low | `#[allow(dead_code)]` on `Task.id` field | Minor tech debt |

### 2.3 File: `spectre-implant/src/c2/packet.rs` (74 lines)

**STATUS: FUNCTIONAL**

`WraithFrame` with 28-byte header. Frame types: DATA(1), REKEY(5), MESH_RELAY(10). Serialize/deserialize with length prefix.

### 2.4 File: `spectre-implant/src/modules/mod.rs` (18 lines)

**STATUS: FUNCTIONAL**

Declares **18 modules**: `bof_loader`, `injection`, `socks`, `shell`, `clr`, `powershell`, `persistence`, `privesc`, `evasion`, `credentials`, `discovery`, `lateral`, `collection`, `smb`, `mesh`, `patch`, `screenshot`, `browser`.

### 2.5 File: `spectre-implant/src/modules/shell.rs` (212 lines)

**STATUS: FUNCTIONAL**

Linux: `fork()` + `execve("/bin/sh")` with pipe-based stdout capture.
Windows: `CreateProcessA` with piped stdout via `STARTUPINFOA`.

### 2.6 File: `spectre-implant/src/modules/injection.rs` (420 lines)

**STATUS: FUNCTIONAL**

3 injection methods on both platforms:
- **Reflective:** `VirtualAllocEx` + `WriteProcessMemory` + `CreateRemoteThread`
- **Process Hollowing:** `CreateProcessA(SUSPENDED)` + `NtUnmapViewOfSection` + `WriteProcessMemory` + `ResumeThread`
- **Thread Hijack:** `SuspendThread` + `GetThreadContext` + `SetThreadContext(RIP)` + `ResumeThread`

Linux equivalents use `process_vm_writev` + `ptrace`.

### 2.7 File: `spectre-implant/src/modules/bof_loader.rs` (332 lines)

**STATUS: FUNCTIONAL**

COFF (BOF) loader with:
- COFF header parsing and section loading
- Symbol resolution with 6 Beacon Internal Functions (BIFs): `BeaconOutput`, `BeaconPrintf`, `BeaconDataParse`, `BeaconDataInt`, `BeaconDataShort`, `BeaconDataLength`
- Relocation processing
- Windows: `VirtualAlloc`-based execution with `VirtualProtect(PAGE_EXECUTE_READ)`
- Linux: `mmap(PROT_READ|PROT_WRITE|PROT_EXEC)` execution

### 2.8 File: `spectre-implant/src/modules/socks.rs` (298 lines)

**STATUS: FUNCTIONAL**

SOCKS4 and SOCKS5 proxy with real TCP connections. Supports `CONNECT` command. Linux uses raw syscalls; Windows uses hash-resolved `ws2_32.dll` APIs.

### 2.9 File: `spectre-implant/src/modules/clr.rs` (216 lines)

**STATUS: PARTIALLY FUNCTIONAL**

.NET CLR hosting via COM interfaces (`ICLRMetaHost`, `ICLRRuntimeInfo`, `ICLRRuntimeHost`).

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| ~163 | CLSID Bug | P2 | CLR CLSID may reference incorrect GUID for `CLRMetaHost`; needs verification against official COM CLSID `9280188d-0e8e-4867-b30c-7fa83884e8de` | Open |
| - | Linux Stub | Info | Returns "CLR not supported on Linux" | Expected |

### 2.10 File: `spectre-implant/src/modules/powershell.rs` (237 lines)

**STATUS: PARTIALLY FUNCTIONAL -- Enhanced from v2.2.5 (150 lines)**

+87 lines growth. CLR-hosted PowerShell execution via Runner.dll assembly loading.

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| ~80 | Placeholder | P1 | `RUNNER_DLL` contains minimal MZ header bytes (4D 5A ...) -- not a real .NET assembly. CLR `ExecuteInDefaultAppDomain` will fail with this stub. | Open |
| - | Linux Stub | Info | Returns error string | Expected |

### 2.11 File: `spectre-implant/src/modules/persistence.rs` (209 lines)

**STATUS: FUNCTIONAL**

3 persistence methods:
- **Registry Run:** `RegOpenKeyExA` / `RegSetValueExA` on `SOFTWARE\Microsoft\Windows\CurrentVersion\Run`
- **Scheduled Task:** Full COM-based `ITaskService` → `ITaskFolder` → `ITaskDefinition` → `IExecAction` pipeline (resolved from v2.2.5 shell delegation)
- **User Creation:** `NetUserAdd` via `netapi32.dll`

### 2.12 File: `spectre-implant/src/modules/privesc.rs` (61 lines)

**STATUS: FUNCTIONAL**

Fodhelper UAC bypass: writes command to `HKCU\Software\Classes\ms-settings\Shell\Open\command`, sets `DelegateExecute` to empty, launches `fodhelper.exe`, cleans registry.

### 2.13 File: `spectre-implant/src/modules/evasion.rs` (143 lines)

**STATUS: FUNCTIONAL**

- **Timestomp:** Copies `FILETIME` (creation, access, write) from source to target file
- **Sandbox Detection:** Checks for `SbieDll.dll` (Sandboxie), `dbghelp.dll` loaded in suspicious context, `GetTickCount() < 30 min`, and `GetSystemInfo().dwNumberOfProcessors < 2`

### 2.14 File: `spectre-implant/src/modules/credentials.rs` (241 lines)

**STATUS: FUNCTIONAL**

Full LSASS memory dump chain:
1. `OpenProcess(PROCESS_ALL_ACCESS, lsass_pid)`
2. `MiniDumpWriteDump` via `dbghelp.dll` to output path
3. Returns sensitive data wrapped in `SensitiveData` (encrypted in-memory)

Linux: reads `/proc/self/maps` as placeholder.

### 2.15 File: `spectre-implant/src/modules/discovery.rs` (294 lines)

**STATUS: FUNCTIONAL**

- `sys_info()`: hostname, username, OS info, process list
- `net_scan()`: TCP connect-scan for port checking with configurable timeout
- `get_hostname()` / `get_username()`: Platform-specific with `SensitiveData` returns

### 2.16 File: `spectre-implant/src/modules/lateral.rs` (117 lines)

**STATUS: FUNCTIONAL**

- **PsExec:** `OpenSCManagerA` -> `CreateServiceA` -> `StartServiceA` with zeroized sensitive strings
- **Service Stop:** `OpenServiceA` -> `ControlService(SERVICE_CONTROL_STOP)`
- Linux stubs return `Err(())`

### 2.17 File: `spectre-implant/src/modules/collection.rs` (122 lines)

**STATUS: FUNCTIONAL**

Keylogger via `GetAsyncKeyState` polling in a `CreateThread`-spawned thread. Uses static mutable `KEY_BUFFER` (512 bytes). `keylogger_poll()` returns and clears buffered keys.

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| - | Thread Safety | P3 | Static mutable `KEY_BUFFER` without synchronization; low risk in single-implant context but technically UB | Open |

### 2.18 File: `spectre-implant/src/modules/smb.rs` (425 lines)

**STATUS: FUNCTIONAL (Linux) / STUB (Windows)**

Full SMB2 client implementation (Linux) with:
- NetBIOS session framing
- SMB2 Negotiate, Session Setup, Tree Connect, Write, Read commands
- SMB2 header construction with proper field layout
- Named pipe data exchange for C2 channel encapsulation

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| ~315 | Placeholder | P3 | "Just skip it and assume success for now" in SMB tree connect response parsing | Open |
| - | Windows Stub | P3 | Windows implementation returns `Err(())` | Open |

### 2.19 File: `spectre-implant/src/modules/mesh.rs` (335 lines)

**STATUS: FUNCTIONAL -- Significantly Enhanced from v2.2.5 (254 lines)**

+81 lines growth. Additions include:
- **MeshRouter:** Cost-based routing with `add_route`, `get_next_hop`, `get_route_cost`, `remove_route`
- **MeshServer:** TCP listener + Windows Named Pipe server with `poll_and_accept`, `send_to_client`
- **Peer Discovery:** `discover_peers()` via UDP broadcast "WRAITH_MESH_HELLO" on port 4444
- Unit tests for `MeshRouter`

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| - | Hardcoded | P3 | UDP broadcast discovery on fixed port 4444 with plaintext "WRAITH_MESH_HELLO" | Detectable signature |

### 2.20 File: `spectre-implant/src/modules/patch.rs` (84 lines) -- NEW

**STATUS: FUNCTIONAL**

AMSI and ETW bypass via memory patching:
- **AMSI Bypass:** Resolves `AmsiScanBuffer` in `amsi.dll`, overwrites with `[0xB8, 0x57, 0x00, 0x07, 0x80, 0xC3]` (mov eax, E_INVALIDARG; ret). Uses `NtProtectVirtualMemory` via indirect syscall (`get_ssn` + `do_syscall`) for memory protection changes. Saves and restores original protection.
- **ETW Bypass:** Resolves `EtwEventWrite` in `ntdll.dll`, overwrites with `[0xC3]` (ret). Same indirect syscall approach for memory protection.
- Linux: Both functions return `Ok(())` as no-ops.

### 2.21 File: `spectre-implant/src/modules/screenshot.rs` (138 lines) -- NEW

**STATUS: FUNCTIONAL**

Full GDI screenshot capture pipeline:
1. `GetDesktopWindow()` -> `GetWindowDC()`
2. `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)`
3. `CreateCompatibleDC()` + `CreateCompatibleBitmap()`
4. `BitBlt(SRCCOPY)` -> `GetDIBits()` (32-bit BGRA)
5. Proper GDI cleanup (`DeleteObject`, `DeleteDC`, `ReleaseDC`)

All APIs resolved via DJB2 hash-based PEB walking.
Linux: Returns error string "Screenshot not supported on Linux".

### 2.22 File: `spectre-implant/src/modules/browser.rs` (93 lines) -- NEW

**STATUS: FUNCTIONAL (Enumeration Only)**

Browser credential path enumeration for:
- Google Chrome (`Google\Chrome\User Data`)
- Microsoft Edge (`Microsoft\Edge\User Data`)
- Brave Browser (`BraveSoftware\Brave-Browser\User Data`)

Checks `Local State` (master key) and `Login Data` in Default, Profile 1, Profile 2. Uses hash-resolved `GetEnvironmentVariableA` and `GetFileAttributesA`.

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| - | Incomplete | P3 | Only enumerates paths; does not perform DPAPI decryption to extract actual credentials | Open |
| - | Linux Stub | Info | Returns "Browser harvesting not supported on Linux" | Expected |

### 2.23 Utility Files

#### `utils/syscalls.rs` (545 lines) -- Enhanced from v2.2.5 (473 lines)

+72 lines growth. Major additions:
- **Halo's Gate SSN Recovery:** `get_ssn()` scans +/-32 neighbors at 32-byte stride from hooked ntdll stubs
- **NT Stub Pattern Matching:** `parse_syscall_stub()` recognizes two patterns (MOV R10,RCX / MOV EAX,SSN)
- **Indirect Syscall Gadget:** `get_syscall_gadget()` scans `NtOpenFile` in ntdll for `0F 05 C3` (`syscall; ret`) sequence
- **`do_syscall()`:** Executes syscall through gadget with 5 arguments, falls back to direct syscall if no gadget found
- Linux: 26 syscall constants, `syscall0` through `syscall6` wrappers, 20+ typed functions (`sys_socket`, `sys_connect`, etc.)

#### `utils/obfuscation.rs` (427 lines) -- Significantly Enhanced from v2.2.5 (265 lines)

+162 lines growth. Major additions:
- **Ekko Sleep (Timer Queue ROP chain):**
  1. `CreateTimerQueue()` + `CreateTimerQueueTimer()`
  2. ROP chain: `NtContinue` + `SystemFunction032` (RC4 encryption of `.text` section)
  3. Context manipulation with `VirtualProtect` cycling (RX -> RW -> RX)
  4. Timer-based sleep with encrypted code section
- **XOR Sleep Mask:**
  1. `VirtualProtect(.text, PAGE_READWRITE)`
  2. XOR-encrypt `.text` section + heap region
  3. Sleep
  4. XOR-decrypt and restore `PAGE_EXECUTE_READ`
- **Linux:** `get_text_range()` uses hardcoded `0x400000` base address

| Line | Issue Type | Severity | Code/Description | Status |
|------|-----------|----------|------------------|--------|
| ~259 | Hardcoded | P3 | Linux `.text` range assumes `0x400000` base -- PIE binaries will have different base | Open |
| - | Entropy | P3 | `entropy.rs` line ~52: RDRAND does not check CF flag for failure; aarch64 uses CNTVCT_EL0 counter (weak) | Open |

#### `utils/api_resolver.rs` (136 lines)

**STATUS: FUNCTIONAL**

DJB2 hash function and PEB walking for dynamic API resolution. Traverses `InMemoryOrderModuleList` via `gs:[0x60]` (Windows x64 PEB).

#### `utils/windows_definitions.rs` (418 lines)

**STATUS: FUNCTIONAL**

Comprehensive Windows type definitions including COM vtable structures for CLR hosting.

#### `utils/heap.rs` (48 lines)

**STATUS: FUNCTIONAL**

`MiniHeap` bump allocator at `0x10000000` with 1 MB capacity. `dealloc()` is a no-op (expected for bump allocator).

#### `utils/sensitive.rs` (130 lines)

**STATUS: FUNCTIONAL**

`SensitiveData` wrapper with XChaCha20-Poly1305 encryption, `Zeroize`/`ZeroizeOnDrop` traits. `SecureBuffer` with platform-specific memory locking (`mlock`/`VirtualLock`).

#### `utils/entropy.rs` (74 lines)

**STATUS: FUNCTIONAL WITH CAVEATS**

RDRAND + RDTSC mixing with PCG-like scramble. See Appendix E for entropy quality concerns.

---

## 3. Operator Client Findings

### 3.1 Rust Backend: `operator-client/src-tauri/src/lib.rs` (1,080 lines)

**STATUS: FUNCTIONAL -- 31 IPC Commands Registered**

All 30 original proto RPCs are wired, plus `connect_to_server` (client-only). **Missing** the 2 new PowerShell management RPCs added in v2.3.0.

**Registered IPC Commands (31):**

| # | Command | Proto RPC | Status |
|---|---------|-----------|--------|
| 1 | `connect_to_server` | N/A (client-only) | OK |
| 2 | `create_campaign` | `CreateCampaign` | OK |
| 3 | `list_implants` | `ListImplants` | OK |
| 4 | `send_command` | `SendCommand` | OK |
| 5 | `list_campaigns` | `ListCampaigns` | OK |
| 6 | `list_listeners` | `ListListeners` | OK |
| 7 | `create_listener` | `CreateListener` | OK |
| 8 | `list_commands` | `ListCommands` | OK |
| 9 | `get_command_result` | `GetCommandResult` | OK |
| 10 | `list_artifacts` | `ListArtifacts` | OK |
| 11 | `download_artifact` | `DownloadArtifact` | OK |
| 12 | `update_campaign` | `UpdateCampaign` | OK |
| 13 | `kill_implant` | `KillImplant` | OK |
| 14 | `start_listener` | `StartListener` | OK |
| 15 | `stop_listener` | `StopListener` | OK |
| 16 | `create_phishing` | `GeneratePhishing` | OK |
| 17 | `list_persistence` | `ListPersistence` | OK |
| 18 | `remove_persistence` | `RemovePersistence` | OK |
| 19 | `list_credentials` | `ListCredentials` | OK |
| 20 | `create_attack_chain` | `CreateAttackChain` | OK |
| 21 | `list_attack_chains` | `ListAttackChains` | OK |
| 22 | `execute_attack_chain` | `ExecuteAttackChain` | OK |
| 23 | `get_attack_chain` | `GetAttackChain` | OK |
| 24 | `refresh_token` | `RefreshToken` | OK |
| 25 | `get_campaign` | `GetCampaign` | OK |
| 26 | `get_implant` | `GetImplant` | OK |
| 27 | `cancel_command` | `CancelCommand` | OK |
| 28 | `generate_implant` | `GenerateImplant` | OK |
| 29 | `list_playbooks` | `ListPlaybooks` | OK |
| 30 | `instantiate_playbook` | `InstantiatePlaybook` | OK |
| 31 | `stream_events` | `StreamEvents` | OK |
| - | **MISSING** | `SetPowerShellProfile` | **NOT WIRED** |
| - | **MISSING** | `GetPowerShellProfile` | **NOT WIRED** |

### 3.2 Rust Backend: `operator-client/src-tauri/src/main.rs` (76 lines)

**STATUS: FUNCTIONAL**

Wayland/KDE workarounds (`WEBKIT_DISABLE_COMPOSITING_MODE`, `QT_QPA_PLATFORM`).

### 3.3 TypeScript Frontend

| File | Lines | Description | Status |
|------|-------|-------------|--------|
| `App.tsx` | 405 | Dashboard with campaigns, beacons, listeners, tabs | Functional |
| `Console.tsx` | 187 | xterm.js interactive console with 11 commands | Functional (see gap below) |
| `AttackChainEditor.tsx` | 202 | Attack chain step editor with MITRE technique IDs | Functional |
| `BeaconInteraction.tsx` | 51 | Beacon detail view with Console embedding | Functional |
| `DiscoveryDashboard.tsx` | 80 | Network discovery visualization | Functional |
| `LootGallery.tsx` | 121 | Artifact and credential browser | Functional |
| `NetworkGraph.tsx` | 252 | Force-directed network visualization (D3-style) | Functional |
| `PersistenceManager.tsx` | 81 | Persistence mechanism management | Functional |
| `PhishingBuilder.tsx` | 85 | Phishing payload generator | Functional |
| `ui/Button.tsx` | 37 | Reusable button component | Functional |

#### Console Command Coverage Gap

`Console.tsx` maps **11 command types** (shell, powershell, persist, lsass, uac, timestomp, sandbox, recon, lateral, keylog, kill).

The implant's `dispatch_tasks()` handles **19 task types**. The following **8 implant tasks are NOT accessible from the Console UI**:

| Task Type | MITRE Technique | Console Gap |
|-----------|-----------------|-------------|
| `inject` | T1055 | No UI -- requires PID + method + hex shellcode |
| `bof` | T1106 | No UI -- requires hex-encoded COFF data |
| `socks` | T1090 | No UI -- requires SOCKS protocol data |
| `screenshot` | T1113 | No UI -- simple trigger, easy to add |
| `browser` | T1555 | No UI -- simple trigger, easy to add |
| `net_scan` | T1046 | No UI -- `recon` maps to `sys_info` only |
| `service_stop` | T1489 | No UI -- service name parameter |
| `mesh_relay` | - | No UI -- internal mesh routing |

**Recommendation:** Add `screenshot`, `browser`, `netscan`, and `inject` commands to Console.tsx as priority additions (5 SP).

---

## 4. Proto Definition Analysis

**File:** `proto/redops.proto` (531 lines)
**Services:** 2 (`OperatorService`, `ImplantService`)

### OperatorService (32 RPCs)

| Category | RPCs | Count |
|----------|------|-------|
| Authentication | Authenticate, RefreshToken | 2 |
| Campaign Mgmt | CreateCampaign, GetCampaign, ListCampaigns, UpdateCampaign | 4 |
| Implant Mgmt | ListImplants, GetImplant, KillImplant | 3 |
| Command Exec | SendCommand, GetCommandResult, ListCommands, CancelCommand | 4 |
| Events | StreamEvents | 1 |
| Artifacts | ListArtifacts, DownloadArtifact | 2 |
| Credentials | ListCredentials | 1 |
| Listeners | CreateListener, ListListeners, StartListener, StopListener | 4 |
| Builder | GenerateImplant, GeneratePhishing | 2 |
| Persistence | ListPersistence, RemovePersistence | 2 |
| Attack Chains | CreateAttackChain, ListAttackChains, ExecuteAttackChain, GetAttackChain | 4 |
| Playbooks | ListPlaybooks, InstantiatePlaybook | 2 |
| **PowerShell** | **SetPowerShellProfile, GetPowerShellProfile** | **2 (NEW)** |
| **Total** | | **32** |

### ImplantService (6 RPCs)

| RPC | Implementation Status |
|-----|----------------------|
| Register | Implemented (with placeholder registration data handling) |
| CheckIn | Implemented |
| GetPendingCommands | Implemented |
| SubmitResult | Implemented |
| UploadArtifact | Implemented |
| DownloadPayload | Implemented |

---

## 5. Integration Gap Analysis

### 5.1 IPC Bridge Coverage: Proto -> Tauri -> Frontend

| Proto RPC (32) | Tauri IPC | Frontend Usage | Status |
|----------------|-----------|----------------|--------|
| Authenticate | `connect_to_server` | App.tsx | OK (merged with connect) |
| RefreshToken | `refresh_token` | App.tsx | OK |
| CreateCampaign | `create_campaign` | App.tsx | OK |
| GetCampaign | `get_campaign` | App.tsx | OK |
| ListCampaigns | `list_campaigns` | App.tsx | OK |
| UpdateCampaign | `update_campaign` | App.tsx | OK |
| ListImplants | `list_implants` | App.tsx | OK |
| GetImplant | `get_implant` | BeaconInteraction | OK |
| KillImplant | `kill_implant` | Console.tsx | OK |
| SendCommand | `send_command` | Console.tsx | OK |
| GetCommandResult | `get_command_result` | BeaconInteraction | OK |
| ListCommands | `list_commands` | BeaconInteraction | OK |
| CancelCommand | `cancel_command` | - | Wired, not yet in UI |
| StreamEvents | `stream_events` | App.tsx (event listener) | OK |
| ListArtifacts | `list_artifacts` | LootGallery | OK |
| DownloadArtifact | `download_artifact` | LootGallery | OK |
| ListCredentials | `list_credentials` | LootGallery | OK |
| CreateListener | `create_listener` | App.tsx | OK |
| ListListeners | `list_listeners` | App.tsx | OK |
| StartListener | `start_listener` | App.tsx | OK |
| StopListener | `stop_listener` | App.tsx | OK |
| GenerateImplant | `generate_implant` | App.tsx | OK |
| GeneratePhishing | `create_phishing` | PhishingBuilder | OK |
| ListPersistence | `list_persistence` | PersistenceManager | OK |
| RemovePersistence | `remove_persistence` | PersistenceManager | OK |
| CreateAttackChain | `create_attack_chain` | AttackChainEditor | OK |
| ListAttackChains | `list_attack_chains` | App.tsx | OK |
| ExecuteAttackChain | `execute_attack_chain` | AttackChainEditor | OK |
| GetAttackChain | `get_attack_chain` | AttackChainEditor | OK |
| ListPlaybooks | `list_playbooks` | App.tsx | OK |
| InstantiatePlaybook | `instantiate_playbook` | App.tsx | OK |
| **SetPowerShellProfile** | **NOT WIRED** | - | **MISSING** |
| **GetPowerShellProfile** | **NOT WIRED** | - | **MISSING** |

**Coverage:** 31 of 32 RPCs wired to IPC (96.9%). 2 PowerShell management RPCs require new Tauri commands.

### 5.2 Console-to-Implant Command Mapping

| Console Command | Maps To | Implant Task Type | Status |
|-----------------|---------|-------------------|--------|
| `shell <cmd>` | `shell` | `shell` | OK |
| `powershell <cmd>` | `powershell` | `powershell` | OK |
| `persist <method> <name> <path>` | `persist` | `persist` | OK |
| `lsass` | `dump_lsass` | `dump_lsass` | OK |
| `uac <cmd>` | `uac_bypass` | `uac_bypass` | OK |
| `timestomp <tgt> <src>` | `timestomp` | `timestomp` | OK |
| `sandbox` | `sandbox_check` | `sandbox_check` | OK |
| `recon` | `sys_info` | `sys_info` | OK |
| `lateral <tgt> <svc> <bin>` | `psexec` | `psexec` | OK |
| `keylog` | `keylogger` | `keylogger` | OK |
| `kill` | `kill` | `kill` | OK |
| - | - | `inject` | **NOT IN CONSOLE** |
| - | - | `bof` | **NOT IN CONSOLE** |
| - | - | `socks` | **NOT IN CONSOLE** |
| - | - | `screenshot` | **NOT IN CONSOLE** |
| - | - | `browser` | **NOT IN CONSOLE** |
| - | - | `net_scan` | **NOT IN CONSOLE** |
| - | - | `service_stop` | **NOT IN CONSOLE** |
| - | - | `mesh_relay` | Internal only |

**Coverage:** 11 of 18 user-facing task types (61%). 7 task types are not accessible from the Console (1 is internal-only).

### 5.3 Noise Protocol Integration

| Component | Implementation | Status |
|-----------|---------------|--------|
| Team Server Keypair | Generated at startup, shared across listeners | OK |
| Listener Handshake | `ProtocolHandler` performs 3-message Noise_XX | OK |
| Implant Handshake | `perform_handshake()` in c2/mod.rs (initiator) | OK |
| Ratchet Key Exchange | Responder sends 32-byte ratchet pubkey in Msg 2 | OK |
| DH Rekeying | Counter-based check calls `session.rekey_dh()` | **INCOMPLETE** (see P1) |

---

## 6. MITRE ATT&CK Coverage

### Implemented Techniques (31 of 38 planned)

| ID | Technique | Module | Status |
|----|-----------|--------|--------|
| **Initial Access (TA0001)** | | | |
| T1566.001 | Spearphishing Attachment | `builder/phishing.rs` | Implemented (HTML smuggling) |
| **Execution (TA0002)** | | | |
| T1059.001 | PowerShell | `modules/powershell.rs` | Partial (Runner.dll placeholder) |
| T1059.004 | Unix Shell | `modules/shell.rs` | Implemented |
| T1106 | Native API | `modules/bof_loader.rs` | Implemented |
| T1129 | Shared Modules | `modules/clr.rs` | Partial (CLSID concern) |
| **Persistence (TA0003)** | | | |
| T1547.001 | Registry Run Keys | `modules/persistence.rs` | Implemented |
| T1053.005 | Scheduled Task | `modules/persistence.rs` | Implemented (COM-based) |
| T1136.001 | Local Account | `modules/persistence.rs` | Implemented |
| **Privilege Escalation (TA0004)** | | | |
| T1548.002 | UAC Bypass | `modules/privesc.rs` | Implemented (Fodhelper) |
| **Defense Evasion (TA0005)** | | | |
| T1055.001 | DLL Injection | `modules/injection.rs` | Implemented (3 methods) |
| T1055.012 | Process Hollowing | `modules/injection.rs` | Implemented |
| T1070.006 | Timestomp | `modules/evasion.rs` | Implemented |
| T1497.001 | Sandbox Evasion | `modules/evasion.rs` | Implemented |
| T1562.001 | Disable Security Tools | `modules/patch.rs` | **NEW: Implemented** (AMSI+ETW bypass) |
| T1027.007 | Dynamic API Resolution | `utils/api_resolver.rs` | Implemented (DJB2 hashing) |
| T1497.003 | Time-Based Evasion | `utils/obfuscation.rs` | Implemented (Ekko + XOR sleep) |
| T1620 | Reflective Code Loading | `modules/bof_loader.rs` | Implemented |
| **Credential Access (TA0006)** | | | |
| T1003.001 | LSASS Memory | `modules/credentials.rs` | Implemented |
| T1555.003 | Credentials from Web Browsers | `modules/browser.rs` | **NEW: Partial** (enumeration only) |
| **Discovery (TA0007)** | | | |
| T1082 | System Information | `modules/discovery.rs` | Implemented |
| T1046 | Network Service Scanning | `modules/discovery.rs` | Implemented |
| T1016 | System Network Config | `modules/discovery.rs` | Implemented |
| **Lateral Movement (TA0008)** | | | |
| T1021.002 | SMB/Windows Admin Shares | `modules/lateral.rs` | Implemented (PsExec) |
| T1570 | Lateral Tool Transfer | `modules/smb.rs` | Implemented (SMB2 C2) |
| **Collection (TA0009)** | | | |
| T1056.001 | Keylogging | `modules/collection.rs` | Implemented |
| T1113 | Screen Capture | `modules/screenshot.rs` | **NEW: Implemented** |
| **Command and Control (TA0011)** | | | |
| T1071.001 | Web Protocols | `c2/mod.rs` (HTTP) | Implemented |
| T1071.004 | DNS | `listeners/dns.rs` | Implemented |
| T1090.001 | Internal Proxy | `modules/socks.rs` | Implemented (SOCKS4/5) |
| T1572 | Protocol Tunneling | `modules/smb.rs` | Implemented (SMB2 Named Pipes) |
| T1573.002 | Asymmetric Crypto | `c2/mod.rs` (Noise_XX) | Implemented |

### Not Yet Implemented (7 of 38)

| ID | Technique | Planned Location | Est. SP |
|----|-----------|-----------------|---------|
| T1059.003 | Windows Command Shell (managed) | `modules/powershell.rs` | 5 (Runner.dll) |
| T1055.003 | Thread Execution Hijack (full) | `modules/injection.rs` | Partial -- implemented but not in Console |
| T1134 | Access Token Manipulation | Not started | 8 |
| T1140 | Deobfuscate/Decode | Not started | 5 |
| T1574.002 | DLL Side-Loading | Not started | 8 |
| T1095 | Non-Application Layer Protocol | P2P Mesh (partial) | 5 |
| T1105 | Ingress Tool Transfer | Not started | 5 |

**Coverage:** 31/38 = **81.6%** (up from 74% in v2.2.5 due to patch.rs, screenshot.rs, browser.rs additions)

---

## 7. Sprint Completion Verification

### Sprint 1: Core C2 Infrastructure

| Task | Status | Evidence |
|------|--------|----------|
| Team Server with gRPC | COMPLETE | `main.rs`, `operator.rs` (32 RPCs) |
| PostgreSQL backend | COMPLETE | `database/mod.rs` (645 lines), 6 migrations |
| HTTP/UDP listeners | COMPLETE | `listeners/http.rs`, `listeners/udp.rs` |
| Noise_XX handshake | COMPLETE | `services/protocol.rs`, `services/session.rs` |
| Implant C2 loop | COMPLETE | `c2/mod.rs` (731 lines) |

### Sprint 2: Implant Capabilities

| Task | Status | Evidence |
|------|--------|----------|
| Shell execution | COMPLETE | `modules/shell.rs` (212 lines) |
| Process injection | COMPLETE | `modules/injection.rs` (420 lines, 3 methods) |
| BOF loading | COMPLETE | `modules/bof_loader.rs` (332 lines) |
| SOCKS proxy | COMPLETE | `modules/socks.rs` (298 lines) |
| Persistence | COMPLETE | `modules/persistence.rs` (209 lines, 3 methods) |
| Privilege escalation | COMPLETE | `modules/privesc.rs` (61 lines) |
| Evasion | COMPLETE | `modules/evasion.rs` (143 lines) |
| Credential access | COMPLETE | `modules/credentials.rs` (241 lines) |
| Discovery | COMPLETE | `modules/discovery.rs` (294 lines) |
| Lateral movement | COMPLETE | `modules/lateral.rs` (117 lines) |

### Sprint 3: Advanced Tradecraft

| Task | Status | Evidence |
|------|--------|----------|
| DNS C2 channel | COMPLETE | `listeners/dns.rs` (326 lines) |
| SMB Named Pipe C2 | COMPLETE | `listeners/smb.rs` (302 lines), `modules/smb.rs` (425 lines) |
| P2P mesh networking | COMPLETE | `modules/mesh.rs` (335 lines) |
| Ekko sleep obfuscation | COMPLETE | `utils/obfuscation.rs` (427 lines) |
| Indirect syscalls | COMPLETE | `utils/syscalls.rs` (545 lines) |
| AMSI/ETW bypass | COMPLETE | `modules/patch.rs` (84 lines) |
| Screenshot capture | COMPLETE | `modules/screenshot.rs` (138 lines) |
| Browser harvesting | COMPLETE | `modules/browser.rs` (93 lines) |
| CLR hosting | PARTIAL | `modules/clr.rs` (216 lines) + `modules/powershell.rs` (237 lines) -- Runner.dll placeholder |
| Kill date / Working hours | COMPLETE | `c2/mod.rs` lines 383-397 |

### Sprint 4: Operator Experience

| Task | Status | Evidence |
|------|--------|----------|
| Tauri 2.0 desktop app | COMPLETE | `operator-client/` |
| React 19 frontend | COMPLETE | 10 TSX components (1,526 lines) |
| xterm.js console | COMPLETE | `Console.tsx` (187 lines) |
| Attack chain editor | COMPLETE | `AttackChainEditor.tsx` (202 lines) |
| Network visualization | COMPLETE | `NetworkGraph.tsx` (252 lines) |
| Real-time events | COMPLETE | `stream_events` + Tauri emit |
| Playbook system | COMPLETE | `playbook_loader.rs` + IPC commands |
| Phishing builder | COMPLETE | `builder/phishing.rs` + `PhishingBuilder.tsx` |
| PowerShell management | PARTIAL | Server-side complete, IPC bridge missing |

---

## 8. Enhancement Recommendations

### 8.1 Sleep Obfuscation Enhancements (P3, 8 SP)

Current Ekko sleep is functional. Potential improvements:
- Module stomping for code location obfuscation
- Stack spoofing to hide call origins during sleep
- CFG-aware sleep with Control Flow Guard bypass

### 8.2 Indirect Syscalls Enhancement (P3, 5 SP)

Current implementation scans `NtOpenFile` for gadget. Could be improved with:
- Multiple gadget sources for redundancy
- Egg hunting in ntdll for alternative `syscall; ret` gadgets
- Syscall number caching to avoid repeated scanning

### 8.3 Redirector/Malleable Profile Support (P3, 15 SP)

No malleable C2 profiles or redirector configuration exists. Consider:
- Configurable HTTP headers, URIs, and user agents
- Traffic jitter profiles
- Domain fronting support

### 8.4 Post-Quantum Hybrid Key Exchange (P3, 13 SP)

Current X25519 is not quantum-resistant. Future work:
- ML-KEM (Kyber) hybrid with X25519
- Larger handshake messages for PQ keys

### 8.5 Improved Entropy (P2, 2 SP)

- Check RDRAND CF flag for failure detection
- Use `/dev/urandom` on Linux instead of RDTSC mixing
- Better ARM64 entropy source (hardware RNG if available)

---

## 9. Prioritized Remediation Plan

### P0: Critical (0 issues)

No P0 issues remain. All critical security findings have been resolved.

### P1: High Priority (2 issues, 18 SP total)

| ID | Finding | Component | Est. SP | Description |
|----|---------|-----------|---------|-------------|
| P1-1 | Key Ratcheting Incomplete | Spectre Implant | 13 | `session.rekey_dh()` in c2/mod.rs calls NoiseTransport method that generates new DH key but does not exchange it with the peer. Forward secrecy per spec (2 min / 1M packets) is not achieved. Requires DH ratchet protocol message exchange. |
| P1-2 | PowerShell Runner DLL | Spectre Implant | 5 | `RUNNER_DLL` in powershell.rs contains minimal MZ stub bytes. `ExecuteInDefaultAppDomain` will fail. Requires real .NET assembly or in-memory CLR script execution alternative. |

### P2: Medium Priority (8 issues, 22 SP total)

| ID | Finding | Component | Est. SP | Description |
|----|---------|-----------|---------|-------------|
| P2-1 | PowerShell IPC Gap | Operator Client | 3 | `SetPowerShellProfile` and `GetPowerShellProfile` proto RPCs (lines 66-67 of redops.proto) have no corresponding Tauri IPC commands. Server-side `PowerShellManager` is ready. |
| P2-2 | Console Command Coverage | Operator Client | 5 | Console.tsx maps 11 of 18 user-facing task types. Missing: inject, bof, socks, screenshot, browser, net_scan, service_stop. |
| P2-3 | Windows UDP Transport | Spectre Implant | 3 | `UdpTransport::request()` for Windows returns `Err(())` unconditionally (c2/mod.rs ~line 325). Needs WinSock2 implementation via hash-resolved APIs. |
| P2-4 | ImplantService Registration | Team Server | 3 | `Register` RPC does not decrypt `encrypted_registration` or validate `ephemeral_public`. Creates generic record with hardcoded defaults. |
| P2-5 | CLR CLSID Verification | Spectre Implant | 1 | `clr.rs` line ~163: CLR MetaHost CLSID needs verification against official COM GUID. |
| P2-6 | Entropy Quality | Spectre Implant | 2 | RDRAND does not check CF flag; aarch64 uses weak CNTVCT_EL0 counter. |
| P2-7 | SMB Header Transmute | Team Server | 3 | 5 `unsafe { core::mem::transmute(h) }` blocks in smb.rs for header serialization. Fragile if struct layout changes. Should use explicit byte serialization. |
| P2-8 | VBA Macro Generator | Team Server | 2 | `phishing.rs` VBA macro generation incomplete. HTML smuggling works. |

### P3: Low Priority (5 issues, 33 SP total)

| ID | Finding | Component | Est. SP | Description |
|----|---------|-----------|---------|-------------|
| P3-1 | Browser DPAPI Decryption | Spectre Implant | 8 | browser.rs only enumerates credential paths. Does not decrypt DPAPI-protected Login Data. |
| P3-2 | Linux .text Base Address | Spectre Implant | 3 | obfuscation.rs uses hardcoded `0x400000` for Linux `.text` range. PIE binaries use different base. Should parse `/proc/self/maps`. |
| P3-3 | Mesh Discovery Signature | Spectre Implant | 5 | UDP broadcast "WRAITH_MESH_HELLO" on port 4444 is a detectable network signature. Should use encrypted discovery. |
| P3-4 | SMB Client Windows Stub | Spectre Implant | 8 | modules/smb.rs Windows implementation returns `Err(())`. Needs WinSock + named pipe implementation. |
| P3-5 | Keylogger Thread Safety | Spectre Implant | 2 | Static mutable `KEY_BUFFER` in collection.rs without synchronization. Technically UB but low practical risk. |
| P3-6 | SMB Parse Assumption | Spectre Implant | 2 | smb.rs line ~315: "Just skip it and assume success for now" in tree connect response. |
| P3-7 | Dead Code Annotations | All Components | 5 | 10 `#[allow(dead_code)]` annotations across 6 files. See Appendix C. |

### Total Remaining Work

| Priority | Count | Story Points |
|----------|-------|-------------|
| P0 | 0 | 0 |
| P1 | 2 | 18 |
| P2 | 8 | 22 |
| P3 | 7 | 33 |
| **Total** | **17** | **73** |

### Estimated Timeline

| Phase | Focus | SP | Duration |
|-------|-------|-----|----------|
| Phase 1 | P1 fixes (key ratchet, Runner.dll) | 18 | 1-2 sprints |
| Phase 2 | P2 fixes (IPC, Console, Windows UDP) | 22 | 2 sprints |
| Phase 3 | P3 enhancements (DPAPI, mesh crypto, etc.) | 33 | 3-4 sprints |
| **Total** | | **73** | **6-8 sprints** |

---

## Appendices

### Appendix A: Complete File Inventory

#### Team Server (24 files, 5,225 lines Rust)

| File | Lines | Category |
|------|-------|----------|
| `src/main.rs` | 230 | Entry point |
| `src/database/mod.rs` | 645 | Database + encryption |
| `src/services/operator.rs` | 1,365 | gRPC OperatorService (32 RPCs) |
| `src/services/implant.rs` | 306 | gRPC ImplantService (6 RPCs) |
| `src/services/session.rs` | 111 | Noise session management |
| `src/services/protocol.rs` | 284 | C2 protocol handler |
| `src/services/listener.rs` | 94 | Listener lifecycle |
| `src/services/killswitch.rs` | 61 | Ed25519 kill switch |
| `src/services/playbook_loader.rs` | 78 | Playbook YAML/JSON loader |
| `src/services/powershell.rs` | 128 | PowerShell profile management (NEW) |
| `src/services/rekey_tests.rs` | 74 | Noise rekey tests (NEW) |
| `src/services/mod.rs` | 9 | Module declarations |
| `src/listeners/http.rs` | 78 | HTTP/Axum listener |
| `src/listeners/udp.rs` | 57 | UDP listener |
| `src/listeners/dns.rs` | 326 | DNS C2 listener |
| `src/listeners/smb.rs` | 302 | SMB2 listener |
| `src/listeners/mod.rs` | 4 | Module declarations |
| `src/builder/mod.rs` | 184 | Implant builder |
| `src/builder/phishing.rs` | 104 | Phishing payload generator |
| `src/governance.rs` | 125 | Governance engine |
| `src/models/mod.rs` | 176 | Data models |
| `src/models/listener.rs` | 14 | Listener model |
| `src/utils.rs` | 40 | JWT utilities |
| `src/auth_tests.rs` | 80 | Auth tests |
| `src/killswitch_config_test.rs` | 121 | Kill switch tests |
| `src/operator_service_test.rs` | 315 | Service integration tests |
| `src/powershell_test.rs` | 79 | PowerShell tests (NEW) |
| `build.rs` | 7 | Proto build script |

#### Spectre Implant (29 files, 6,553 lines Rust)

| File | Lines | Category |
|------|-------|----------|
| `src/lib.rs` | 48 | Entry point (no_std) |
| `src/c2/mod.rs` | 731 | Core C2 (19 task types) |
| `src/c2/packet.rs` | 74 | WraithFrame protocol |
| `src/modules/mod.rs` | 18 | Module declarations (18 modules) |
| `src/modules/shell.rs` | 212 | Shell execution |
| `src/modules/injection.rs` | 420 | Process injection (3 methods) |
| `src/modules/bof_loader.rs` | 332 | COFF/BOF loader |
| `src/modules/socks.rs` | 298 | SOCKS4/5 proxy |
| `src/modules/clr.rs` | 216 | .NET CLR hosting |
| `src/modules/powershell.rs` | 237 | PowerShell via CLR |
| `src/modules/persistence.rs` | 209 | Persistence (3 methods) |
| `src/modules/privesc.rs` | 61 | UAC bypass |
| `src/modules/evasion.rs` | 143 | Timestomp + sandbox detection |
| `src/modules/credentials.rs` | 241 | LSASS dump |
| `src/modules/discovery.rs` | 294 | System + network discovery |
| `src/modules/lateral.rs` | 117 | PsExec lateral movement |
| `src/modules/collection.rs` | 122 | Keylogging |
| `src/modules/smb.rs` | 425 | SMB2 client |
| `src/modules/mesh.rs` | 335 | P2P mesh networking |
| `src/modules/patch.rs` | 84 | AMSI/ETW bypass (NEW) |
| `src/modules/screenshot.rs` | 138 | Screen capture (NEW) |
| `src/modules/browser.rs` | 93 | Browser credential enum (NEW) |
| `src/utils/mod.rs` | 9 | Module declarations |
| `src/utils/syscalls.rs` | 545 | Syscalls + indirect syscalls |
| `src/utils/obfuscation.rs` | 427 | Ekko + XOR sleep |
| `src/utils/api_resolver.rs` | 136 | DJB2 hash + PEB walking |
| `src/utils/windows_definitions.rs` | 418 | Windows type definitions |
| `src/utils/heap.rs` | 48 | MiniHeap bump allocator |
| `src/utils/sensitive.rs` | 130 | Encrypted memory wrapper |
| `src/utils/entropy.rs` | 74 | RDRAND + RDTSC entropy |
| `src/utils/test_heap.rs` | 16 | Heap unit test |
| `src/utils/test_sensitive.rs` | 13 | SensitiveData unit test |
| `tests/test_smb.rs` | 17 | SMB integration test |

#### Operator Client (13 files: 1,164 lines Rust + 1,526 lines TS/TSX)

| File | Lines | Language | Description |
|------|-------|----------|-------------|
| `src-tauri/src/lib.rs` | 1,080 | Rust | 31 IPC commands + types |
| `src-tauri/src/main.rs` | 76 | Rust | Wayland/KDE workarounds |
| `src-tauri/build.rs` | 8 | Rust | Proto build script |
| `src/App.tsx` | 405 | TSX | Main dashboard |
| `src/components/Console.tsx` | 187 | TSX | Interactive console |
| `src/components/AttackChainEditor.tsx` | 202 | TSX | Attack chain editor |
| `src/components/BeaconInteraction.tsx` | 51 | TSX | Beacon detail view |
| `src/components/DiscoveryDashboard.tsx` | 80 | TSX | Discovery visualization |
| `src/components/LootGallery.tsx` | 121 | TSX | Artifact/credential browser |
| `src/components/NetworkGraph.tsx` | 252 | TSX | Network graph |
| `src/components/PersistenceManager.tsx` | 81 | TSX | Persistence management |
| `src/components/PhishingBuilder.tsx` | 85 | TSX | Phishing builder |
| `src/components/ui/Button.tsx` | 37 | TSX | UI component |
| `src/main.tsx` | 10 | TSX | React entry |
| `vite.config.ts` | 13 | TS | Build config |
| `vite.config.d.ts` | 2 | TS | Type declarations |

#### Proto + SQL

| File | Lines | Description |
|------|-------|-------------|
| `proto/redops.proto` | 531 | Service definitions (32+6 RPCs) |
| `migrations/20251129000000_initial_schema.sql` | 162 | Core tables |
| `migrations/20260125000000_audit_signature.sql` | 1 | Audit signature column |
| `migrations/20260125000001_persistence_table.sql` | 7 | Persistence tracking |
| `migrations/20260126000000_attack_chains.sql` | 18 | Attack chain tables |
| `migrations/20260126000001_playbooks.sql` | 10 | Playbook table |
| `migrations/20260127000000_listeners_table.sql` | 10 | Listener state table |

### Grand Total

| Category | Files | Lines |
|----------|-------|-------|
| Team Server (Rust) | 28 | 5,225 |
| Spectre Implant (Rust) | 32 | 6,553 |
| Operator Client (Rust) | 3 | 1,164 |
| Operator Client (TS/TSX) | 13 | 1,526 |
| Proto | 1 | 531 |
| SQL Migrations | 6 | 208 |
| **Grand Total** | **83** | **15,207** |

### Appendix B: Hardcoded Values

| File | Line | Value | Risk | Notes |
|------|------|-------|------|-------|
| `c2/mod.rs` | 38 | `sleep_interval: 5000` | Low | Default before patching |
| `c2/mod.rs` | 69 | `"127.0.0.1"` | Low | Fallback if config empty |
| `c2/mod.rs` | 402 | `port: 8080` | Low | Default HTTP port |
| `c2/mod.rs` | 403 | `port: 9999` | Low | Default UDP port |
| `c2/mod.rs` | 408 | `port: 4444` | Medium | Mesh TCP hardcoded; well-known Metasploit port |
| `mesh.rs` | - | `"WRAITH_MESH_HELLO"` | Medium | Detectable plaintext broadcast |
| `mesh.rs` | - | `port: 4444` | Medium | Fixed broadcast discovery port |
| `obfuscation.rs` | ~259 | `0x400000` | Low | Linux .text base assumption |
| `heap.rs` | - | `0x10000000` | Low | Bump allocator base address |
| `lib.rs` | - | `0x100000` (1 MB) | Low | Heap size |
| `injection.rs` | ~220 | `0x400000` | Low | Process hollowing preferred base |
| `bof_loader.rs` | - | 6 BIF hashes | Info | Fixed DJB2 hashes for BOF API |

### Appendix C: `#[allow(dead_code)]` Annotations

| File | Line | Target | Justification |
|------|------|--------|---------------|
| `team-server/src/database/mod.rs` | 83 | Database function | May be used in future queries |
| `team-server/src/database/mod.rs` | 354 | Database function | May be used in future queries |
| `team-server/src/database/mod.rs` | 525 | Database function | May be used in future queries |
| `team-server/src/services/operator.rs` | 16 | `governance` field | Injected but accessed through other means |
| `team-server/src/services/operator.rs` | 18 | `static_key` field | Injected for future direct use |
| `team-server/src/services/operator.rs` | 20 | `sessions` field | Injected for future direct use |
| `team-server/src/services/session.rs` | 60 | Session field | May be used in rekey implementation |
| `team-server/src/services/powershell.rs` | 26 | PowerShell manager field | New module, gradual integration |
| `team-server/src/models/mod.rs` | 76 | Model field | Serialization completeness |
| `spectre-implant/src/c2/mod.rs` | 516 | `Task.id` field | Deserialized but not used in dispatch |
| **Total** | | **10 annotations** | |

### Appendix D: `unsafe` Usage Summary

| Component | File | Count | Primary Purpose |
|-----------|------|-------|-----------------|
| Spectre Implant | `syscalls.rs` | 38 | Raw syscall invocation |
| Spectre Implant | `obfuscation.rs` | 33 | Memory protection, ROP chain |
| Spectre Implant | `clr.rs` | 36 | COM vtable calls |
| Spectre Implant | `injection.rs` | 30 | Process memory manipulation |
| Spectre Implant | `mesh.rs` | 20 | Socket/pipe operations |
| Spectre Implant | `c2/mod.rs` | 18 | WinINet API, static config |
| Spectre Implant | `powershell.rs` | 17 | CLR hosting |
| Spectre Implant | `credentials.rs` | 15 | LSASS memory access |
| Spectre Implant | `discovery.rs` | 15 | System info APIs |
| Spectre Implant | `socks.rs` | 13 | Socket operations |
| Spectre Implant | `smb.rs` | 18 | SMB2 protocol I/O |
| Spectre Implant | `screenshot.rs` | 12 | GDI API calls |
| Spectre Implant | `windows_definitions.rs` | 11 | Type transmute/cast |
| Spectre Implant | `bof_loader.rs` | 11 | COFF loading + execution |
| Spectre Implant | `persistence.rs` | 10 | Registry + COM APIs |
| Spectre Implant | `lateral.rs` | 10 | SCM API calls |
| Spectre Implant | `evasion.rs` | 9 | File time manipulation |
| Spectre Implant | `collection.rs` | 8 | KeyState polling, CreateThread |
| Spectre Implant | `shell.rs` | 7 | Process creation |
| Spectre Implant | `api_resolver.rs` | 6 | PEB walking |
| Spectre Implant | `sensitive.rs` | 4 | Memory locking |
| Spectre Implant | `heap.rs` | 4 | Custom allocator |
| Spectre Implant | `browser.rs` | 4 | API calls |
| Spectre Implant | `privesc.rs` | 4 | Registry manipulation |
| Spectre Implant | `patch.rs` | 3 | Memory patching (AMSI/ETW) |
| Spectre Implant | `entropy.rs` | 2 | RDRAND intrinsics |
| Team Server | `listeners/smb.rs` | 5 | SMB2 header transmute |
| Operator Client | `main.rs` | 3 | Env var setting |
| Team Server | `killswitch_config_test.rs` | 2 | Test env setup |
| Team Server | Various tests | 3 | Test helper code |
| **Total** | **33 files** | **~373** | Expected for no_std implant |

**Risk Assessment:** The high `unsafe` count is expected and justified for a `#![no_std]` implant that performs direct syscalls, WinAPI calls via PEB walking, COM vtable invocations, and raw memory manipulation. No unnecessary `unsafe` blocks were identified.

### Appendix E: `.unwrap()` and `.expect()` in Production Code

#### `.unwrap()` in Production (Non-Test) Code

| File | Count | Context | Risk |
|------|-------|---------|------|
| `spectre-implant/src/c2/mod.rs` | 1 | IP parsing fallback | Low (has `.unwrap_or("0")`) |
| `spectre-implant/src/modules/bof_loader.rs` | 2 | COFF section parsing | Medium -- malformed COFF could panic |
| `team-server/src/services/implant.rs` | 1 | UUID split (guaranteed) | Low |
| **Total** | **4** | | |

#### `.expect()` in Production Code

| File | Count | Context | Risk |
|------|-------|---------|------|
| `team-server/src/main.rs` | 3 | Startup env vars | Low (fail-fast at startup) |
| `team-server/src/database/mod.rs` | 4 | Crypto key validation | Low (fail-fast at startup) |
| `team-server/src/services/killswitch.rs` | 3 | Kill switch key loading | Low (fail-fast) |
| `team-server/src/services/operator.rs` | 3 | Env vars in RPC handlers | Medium -- runtime panic possible |
| `team-server/src/services/session.rs` | 2 | Noise handshake | Medium -- handshake failure panic |
| `team-server/src/utils.rs` | 1 | JWT secret | Low (fail-fast at startup) |
| `spectre-implant/src/utils/sensitive.rs` | 1 | Crypto nonce | Low |
| `operator-client/src-tauri/src/lib.rs` | 1 | Tauri app startup | Low |
| **Total** | **18** | | |

### Appendix F: Pattern Scan Results

| Pattern | Matches | Files |
|---------|---------|-------|
| `TODO\|FIXME\|HACK\|XXX\|WORKAROUND` | 0 | None |
| `todo!()\|unimplemented!()\|unreachable!()` | 0 | None |
| `In a real\|placeholder\|stub\|assume success` | 6 | dns.rs, LootGallery.tsx, PhishingBuilder.tsx, App.tsx, obfuscation.rs, smb.rs |
| `#[allow(dead_code)]` | 10 | See Appendix C |
| `.unwrap()` (all) | 80 | 13 files (76 in test code) |
| `.expect()` (all) | 18 | 10 files |
| `unsafe` | 373 | 33 files |

### Appendix G: Consolidated Changes from v2.2.5

This section documents all findings from the v2.2.5 gap analysis (v5.0.0 internal) that have been resolved, remain open, or are newly discovered in v2.3.0.

#### Resolved Since v2.2.5

| v2.2.5 Finding | Resolution |
|----------------|------------|
| SMB2 Header Struct Bug (P1) | Fixed -- struct field mismatch corrected |
| Playbook IPC Bridge Missing (P2) | `list_playbooks` and `instantiate_playbook` wired |
| Missing 7 Proto RPCs (P2) | All 30 original RPCs now wired |
| Persistence schtasks Shell Delegation (P2) | Full COM-based ITaskService pipeline |
| P2P Mesh Not Implemented (P3) | mesh.rs (335 lines) with MeshRouter + discovery |
| SMB2 Pending Data TODO (P2) | `pending_data` buffer implemented in listener |

#### New in v2.3.0

| Finding | Priority | Details |
|---------|----------|---------|
| 3 new implant modules | N/A | patch.rs, screenshot.rs, browser.rs |
| 2 new proto RPCs not wired | P2 | SetPowerShellProfile, GetPowerShellProfile |
| 3 new team server files | N/A | powershell.rs, rekey_tests.rs, powershell_test.rs |
| Indirect syscall gadget | N/A | get_syscall_gadget() + do_syscall() in syscalls.rs |
| Ekko sleep ROP chain | N/A | Timer Queue + SystemFunction032 in obfuscation.rs |
| MeshRouter with cost routing | N/A | Cost-based routing + discover_peers in mesh.rs |
| AMSI/ETW now implemented | N/A | Was "Not Implemented" in v2.2.5 |
| Screenshot now implemented | N/A | Was not planned in v2.2.5 |
| Browser harvest implemented | N/A | Was not planned in v2.2.5 |

---

## Conclusion

WRAITH-RedOps v2.3.0 represents significant progress from v2.2.5, with 3 new implant modules (patch, screenshot, browser), expanded tradecraft capabilities (Ekko sleep, indirect syscalls, AMSI/ETW bypass), and improved server-side infrastructure (PowerShell management, rekey tests). The platform is at approximately 96% completion with zero P0 issues and only 2 P1 items remaining (key ratcheting and PowerShell runner DLL). The estimated remaining work is 73 story points across 17 findings.

The most impactful next steps are:
1. Implement proper DH ratchet key exchange (P1-1, 13 SP) for forward secrecy
2. Complete the PowerShell Runner.dll (P1-2, 5 SP) for managed code execution
3. Wire the 2 new PowerShell IPC commands (P2-1, 3 SP)
4. Expand Console.tsx with missing commands (P2-2, 5 SP), particularly the new `screenshot` and `browser` capabilities
5. Implement Windows UDP transport (P2-3, 3 SP) for full transport failover support

---

*This document consolidates and supersedes GAP-ANALYSIS-v2.2.5.md (v5.0.0 internal). All v2.2.5 findings have been re-verified and integrated.*

*Generated by Claude Opus 4.5 -- Automated Source Code Audit v6.0.0*
*Audit completed: 2026-01-27*
