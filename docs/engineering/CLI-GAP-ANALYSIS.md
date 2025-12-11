# WRAITH CLI Gap Analysis & Implementation Plan

**Date:** 2025-12-11
**Version:** 1.5.8
**Analysis Scope:** Complete verification of CLI implementation against documentation

---

## Executive Summary

The WRAITH CLI has a solid foundation with 7 out of 12 documented commands partially or fully implemented. However, there are significant gaps between the comprehensive documentation (docs/cli/) and the actual implementation (crates/wraith-cli/). This analysis identifies 25+ specific gaps requiring implementation.

**Key Findings:**
- ✅ **5 commands fully implemented:** Send, Batch, Receive, Daemon, Info
- ⚠️ **4 commands partially implemented:** Status, Peers, Health, Metrics
- ❌ **2 commands missing:** Ping, Config (with show/set subcommands)
- ❌ **1 command naming mismatch:** `Keygen` vs documented `generate-key`
- ❌ **15+ missing command flags and options**
- ❌ **No runtime IPC mechanism** for status/metrics/health queries

---

## Command-by-Command Analysis

### 1. `daemon` - Start WRAITH Daemon

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ✅ | ✅ COMPLETE |
| `--bind` flag | ✅ | ✅ | ✅ COMPLETE |
| `--relay` flag | ✅ | ✅ | ✅ COMPLETE |
| `--workers` flag | ✅ | ❌ | ❌ MISSING |
| `--cpus` flag (CPU pinning) | ✅ | ❌ | ❌ MISSING |
| Output formatting | ✅ | ⚠️ | ⚠️ PARTIAL |

**Gaps:**
1. Missing `--workers N` flag to specify worker thread count
2. Missing `--cpus 0-3` flag for CPU pinning
3. Output doesn't show all documented fields (XDP zero-copy status, etc.)

---

### 2. `send` - Send File to Peer

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ✅ | ✅ COMPLETE |
| File argument | ✅ | ✅ | ✅ COMPLETE |
| `--to` flag | ✅ | ❌ | ❌ NAMING ISSUE |
| Recipient as positional | ❌ | ✅ | ⚠️ DOCS MISMATCH |
| `--mode` flag | ✅ | ✅ | ✅ COMPLETE |
| Multiple recipients | ✅ | ❌ | ❌ MISSING |
| Progress bar | ✅ | ✅ | ✅ COMPLETE |

**Gaps:**
1. Documentation shows `--to PEER_ID` but implementation uses positional `recipient`
2. Cannot send to multiple peers (docs show `--to` can be specified multiple times)
3. Mode flag exists but isn't connected to obfuscation configuration

---

### 3. `batch` - Send Multiple Files

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ✅ | ✅ COMPLETE |
| `--to` flag | ✅ | ✅ | ✅ COMPLETE |
| `--files` flag | ✅ | ✅ | ✅ COMPLETE |
| `--mode` flag | ✅ | ✅ | ✅ COMPLETE |
| Directory recursion | ✅ | ❌ | ❌ MISSING |
| Progress per file | ✅ | ✅ | ✅ COMPLETE |

**Gaps:**
1. Cannot send directory recursively (`--files ~/Documents/project/`)
2. Mode flag not connected to obfuscation

---

### 4. `receive` - Receive Files

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ✅ | ✅ COMPLETE |
| `--output` flag | ✅ | ✅ | ✅ COMPLETE |
| `--bind` flag | ✅ | ✅ | ✅ COMPLETE |
| `--auto-accept` flag | ✅ | ❌ | ❌ MISSING |
| `--trusted-peers` flag | ✅ | ❌ | ❌ MISSING |
| Interactive prompt | ✅ | ❌ | ❌ MISSING |

**Gaps:**
1. Missing `--auto-accept` flag for automatic file acceptance
2. Missing `--trusted-peers FILE` for whitelist
3. No interactive y/N prompt before receiving (docs show "Accept transfer of report.pdf (10.5 MB)? [y/N]:")

---

### 5. `status` - Show Connection Status

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ✅ | ✅ COMPLETE |
| Basic status | ✅ | ⚠️ | ⚠️ PARTIAL |
| `--detailed` flag | ✅ | ⚠️ | ⚠️ PARSED ONLY |
| `--transfer ID` flag | ✅ | ⚠️ | ⚠️ PARSED ONLY |
| Runtime metrics | ✅ | ❌ | ❌ NO IPC |
| Active transfers | ✅ | ❌ | ❌ NO IPC |
| Session statistics | ✅ | ❌ | ❌ NO IPC |

**Gaps:**
1. `--detailed` flag parsed but doesn't show detailed output
2. `--transfer ID` flag parsed but shows placeholder message
3. No IPC mechanism to query running daemon
4. Shows static config only, not runtime status

---

### 6. `peers` - List Connected Peers

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ✅ | ✅ COMPLETE |
| Basic peer list | ✅ | ❌ | ❌ NO IPC |
| `--dht-query` flag | ✅ | ⚠️ | ⚠️ DIFFERENT |
| `--verbose` flag | ✅ | ❌ | ❌ MISSING |
| Peer statistics | ✅ | ❌ | ❌ NO IPC |

**Gaps:**
1. No basic peer listing (requires running daemon + IPC)
2. `--dht-query` takes peer ID argument, but docs show it as boolean flag
3. Missing `--verbose` for detailed peer info
4. No connection statistics (RTT, packets, loss rate)

---

### 7. `health` - Check Node Health

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ✅ | ✅ COMPLETE |
| Static checks | ✅ | ✅ | ✅ COMPLETE |
| Runtime health | ✅ | ❌ | ❌ NO IPC |
| Component status | ✅ | ❌ | ❌ NO IPC |
| Resource usage | ✅ | ❌ | ❌ NO IPC |
| Warnings/errors | ✅ | ❌ | ❌ NO IPC |

**Gaps:**
1. Only shows static configuration checks
2. No runtime health metrics (CPU, memory, network)
3. No component status (Transport, Discovery, Sessions, Transfers)
4. No warnings or error detection

---

### 8. `metrics` - Display Metrics

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ✅ | ✅ COMPLETE |
| `--json` flag | ✅ | ✅ | ✅ COMPLETE |
| `--watch` flag | ✅ | ⚠️ | ⚠️ PARSED ONLY |
| `--interval N` flag | ✅ | ⚠️ | ⚠️ PARSED AS WATCH PARAM |
| Runtime metrics | ✅ | ❌ | ❌ NO IPC |
| Detailed statistics | ✅ | ❌ | ❌ NO IPC |

**Gaps:**
1. `--watch` flag parsed but not implemented
2. Shows static config in JSON, not runtime metrics
3. No transport layer metrics
4. No session/transfer statistics
5. No BBR congestion control metrics
6. No crypto operation counters

---

### 9. `info` - Show Node Information

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ✅ | ✅ COMPLETE |
| Node ID | ✅ | ✅ | ✅ COMPLETE |
| Public keys | ✅ | ✅ | ✅ COMPLETE |
| Network info | ✅ | ⚠️ | ⚠️ PARTIAL |
| Transport capabilities | ✅ | ⚠️ | ⚠️ PARTIAL |
| Features | ✅ | ✅ | ✅ COMPLETE |
| Configuration | ✅ | ⚠️ | ⚠️ PARTIAL |

**Gaps:**
1. Doesn't show public address (STUN-detected)
2. Doesn't show NAT status
3. Doesn't show network type (IPv4/IPv6)
4. Doesn't show ring sizes, UMEM size
5. Missing NUMA node information

---

### 10. `ping` - Ping a Peer

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ❌ | ❌ MISSING |
| Peer ID argument | ✅ | ❌ | ❌ MISSING |
| `--count N` flag | ✅ | ❌ | ❌ MISSING |
| `--interval MS` flag | ✅ | ❌ | ❌ MISSING |
| RTT statistics | ✅ | ❌ | ❌ MISSING |

**Gaps:**
1. **ENTIRE COMMAND MISSING**
2. Need to implement ICMP-like ping over WRAITH protocol
3. Need statistics (min/avg/max/mdev RTT, packet loss)

---

### 11. `config` - Manage Configuration

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command exists | ✅ | ❌ | ❌ MISSING |
| `show` subcommand | ✅ | ❌ | ❌ MISSING |
| `set` subcommand | ✅ | ❌ | ❌ MISSING |
| `--json` flag (for show) | ✅ | ❌ | ❌ MISSING |
| Nested key access | ✅ | ❌ | ❌ MISSING |

**Gaps:**
1. **ENTIRE COMMAND MISSING**
2. Need `config show` to display current configuration
3. Need `config show --json` for JSON output
4. Need `config set KEY VALUE` to update configuration
5. Need nested key support (e.g., `node.bind`, `obfuscation.padding_mode`)

---

### 12. `generate-key` / `keygen` - Generate Identity

| Feature | Documented | Implemented | Status |
|---------|------------|-------------|--------|
| Command (as `generate-key`) | ✅ | ❌ | ❌ NAMING |
| Command (as `keygen`) | ❌ | ✅ | ⚠️ NAME MISMATCH |
| `--output` flag | ✅ | ✅ | ✅ COMPLETE |
| `--encrypt` flag | ✅ | ⚠️ | ⚠️ ALWAYS ENCRYPTS |
| Passphrase prompt | ✅ | ✅ | ✅ COMPLETE |
| Encrypted output | ✅ | ✅ | ✅ COMPLETE |

**Gaps:**
1. Command named `Keygen` in code but `generate-key` in docs
2. Always encrypts (no `--encrypt` flag check)
3. Should support unencrypted output for testing

---

## Global Flags Analysis

| Flag | Documented | Implemented | Status |
|------|------------|-------------|--------|
| `--verbose` / `-v` | ✅ | ✅ | ✅ COMPLETE |
| `--debug` / `-d` | ✅ | ✅ | ✅ COMPLETE |
| `--help` / `-h` | ✅ | ✅ | ✅ COMPLETE (auto) |
| `--version` / `-V` | ✅ | ⚠️ | ⚠️ AUTO (needs test) |
| `--config FILE` | ✅ | ✅ | ✅ COMPLETE |

**Notes:**
- All global flags implemented
- `--version` is handled by clap automatically but should be tested

---

## Critical Infrastructure Gaps

### IPC Mechanism Missing

**Problem:** Many commands (status, peers, health, metrics) require querying a running daemon, but there's no IPC mechanism implemented.

**Impact:**
- `status` shows static config only
- `peers` can't list active connections
- `health` can't show runtime metrics
- `metrics` can't display real-time stats

**Solutions:**
1. **Unix Domain Socket** - Daemon listens on `/tmp/wraith.sock` or `~/.wraith/daemon.sock`
2. **Named Pipes** - Cross-platform pipe-based IPC
3. **Shared Memory** - For high-performance metric access
4. **HTTP API** - Daemon exposes localhost-only HTTP endpoint

**Recommendation:** Unix Domain Socket (simple, secure, POSIX-standard)

---

## Implementation Priority

### P0 - Critical (Blocker for v1.0.0)

1. **Add `ping` command** - Required for connectivity testing
2. **Add `config` command** - Essential for usability
3. **Fix command naming** - `keygen` → `generate-key` (or update docs)
4. **Implement IPC mechanism** - Foundation for runtime queries

### P1 - High (Important for production)

5. **Enhance `status --detailed`** - Full detailed output
6. **Enhance `peers`** - Basic listing + verbose mode
7. **Enhance `health`** - Runtime health checks
8. **Enhance `metrics --watch`** - Live metrics monitoring
9. **Add `receive` interactive prompts** - `--auto-accept`, `--trusted-peers`
10. **Add `send` multi-peer support** - Send to multiple recipients

### P2 - Medium (Nice to have)

11. **Add `daemon --workers`** - Worker thread configuration
12. **Add `daemon --cpus`** - CPU pinning
13. **Enhance `batch`** - Directory recursion
14. **Enhance `info`** - STUN-detected address, NAT status
15. **Add `generate-key --encrypt` flag** - Optional encryption

### P3 - Low (Future enhancements)

16. **Add shell completion** - Bash/zsh/fish completions
17. **Add man pages** - System manual pages
18. **Add config validation command** - `config validate`
19. **Add update checker** - Check for new releases

---

## Testing Gaps

### Current Test Coverage

- **Total CLI tests:** 7
- **Config module:** ~20 tests
- **Progress module:** ~20 tests
- **Main module:** 16 tests (encryption, path sanitization)

### Missing Test Coverage

1. **No integration tests** - Commands don't actually run end-to-end
2. **No command tests** - Each command should have dedicated tests
3. **No IPC tests** - Once implemented
4. **No error handling tests** - What happens when node unreachable?
5. **No cross-platform tests** - Windows vs Linux differences

**Target:** 100+ tests covering all commands and edge cases

---

## Documentation Gaps

### Code vs Docs Mismatches

1. **Command naming:** `keygen` vs `generate-key`
2. **Flag variations:** `--to` as flag vs positional recipient
3. **DHT query argument:** Takes peer ID vs boolean flag
4. **Version number:** Docs show "0.9.0" but project is v1.5.8

### Documentation Updates Needed

1. Update examples to use `keygen` (or rename command)
2. Clarify `send` recipient syntax
3. Update version references throughout
4. Add IPC implementation notes

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1)

**Goal:** Fix critical gaps and establish IPC

- [ ] Implement IPC mechanism (Unix domain sockets)
- [ ] Add `ping` command with RTT statistics
- [ ] Add `config` command (show/set subcommands)
- [ ] Write tests for new commands (20+ tests)

**Deliverables:**
- IPC working between CLI and daemon
- `ping` functional with statistics
- `config show` and `config set` working
- Test coverage >70% for new code

---

### Phase 2: Enhancements (Week 2)

**Goal:** Complete partial implementations

- [ ] Enhance `status --detailed` with full output
- [ ] Enhance `peers` with listing and verbose mode
- [ ] Enhance `health` with runtime checks
- [ ] Enhance `metrics --watch` with live updates
- [ ] Add `receive` interactive prompts
- [ ] Add `send` multi-peer support
- [ ] Write tests for enhancements (30+ tests)

**Deliverables:**
- All documented features implemented
- Test coverage >80% for CLI module
- All commands produce documented output

---

### Phase 3: Polish (Week 3)

**Goal:** Production-ready quality

- [ ] Add missing daemon flags (`--workers`, `--cpus`)
- [ ] Add `batch` directory recursion
- [ ] Enhance `info` with network details
- [ ] Add `generate-key --encrypt` flag
- [ ] Fix command naming inconsistencies
- [ ] Update all documentation
- [ ] Write comprehensive integration tests (20+ tests)
- [ ] Achieve 90%+ test coverage

**Deliverables:**
- Zero gaps between docs and implementation
- Test coverage >90%
- Documentation fully synchronized
- All quality checks passing

---

## Success Criteria

### Completeness

- ✅ All 12 documented commands implemented
- ✅ All documented flags and options implemented
- ✅ All output formats match documentation
- ✅ Zero TODOs or placeholders in code

### Quality

- ✅ Zero clippy warnings
- ✅ All tests passing (target: 100+ CLI tests)
- ✅ Test coverage >90%
- ✅ Documentation synchronized with code

### Functionality

- ✅ Can send/receive files end-to-end
- ✅ Can ping peers and measure RTT
- ✅ Can query daemon status in real-time
- ✅ Can manage configuration from CLI
- ✅ All interactive prompts working

---

## Risk Assessment

### High Risk

1. **IPC Implementation Complexity**
   - **Risk:** Unix sockets may have platform-specific issues
   - **Mitigation:** Use well-tested libraries (tokio::net::UnixStream)
   - **Fallback:** Named pipes or HTTP API

2. **Testing Daemon Integration**
   - **Risk:** Integration tests require running daemon
   - **Mitigation:** Mock Node API for unit tests
   - **Fallback:** Manual testing documented

### Medium Risk

3. **Breaking Changes to Command Interface**
   - **Risk:** Renaming commands may break user scripts
   - **Mitigation:** Deprecation warnings, alias support
   - **Fallback:** Document changes in migration guide

4. **Performance of `metrics --watch`**
   - **Risk:** High-frequency IPC may impact daemon performance
   - **Mitigation:** Rate-limit updates, async queries
   - **Fallback:** Longer default interval

### Low Risk

5. **Documentation Synchronization**
   - **Risk:** Docs may drift during development
   - **Mitigation:** Update docs alongside code changes
   - **Fallback:** Documentation sprint at end

---

## Conclusion

The WRAITH CLI has a strong foundation but requires significant work to align with its comprehensive documentation. The three-week roadmap above provides a structured approach to close all gaps and deliver a production-ready CLI.

**Estimated Total Effort:** ~120 hours (3 weeks full-time)
- Phase 1 (Foundation): 40 hours
- Phase 2 (Enhancements): 40 hours
- Phase 3 (Polish): 40 hours

**Recommended Approach:** Tackle phases sequentially with quality gates between each phase.

---

**Generated:** 2025-12-11
**Author:** Claude Code
**Review Status:** DRAFT - Pending review and prioritization
