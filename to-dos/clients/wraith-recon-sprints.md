# WRAITH-Recon Client - Sprint Planning (Granular)

**Client Name:** WRAITH-Recon
**Tier:** 3 (Advanced)
**Timeline:** 12 weeks (3 sprints × 4 weeks)
**Total Story Points:** 180

---

## Phase 1: Foundation & Passive Visibility (Weeks 40-43)

### S1.1: Governance & Safety Core (15 pts)
*   [ ] **Task:** Define `RoE` Struct and Serde serialization.
    *   *Acceptance Criteria:* JSON format matches spec; optional fields handled correctly.
*   [ ] **Task:** Implement Ed25519 Signature Verification logic.
    *   *Acceptance Criteria:* Valid signature returns `Ok`; invalid/expired returns `Err`; replay attacks blocked.
*   [ ] **Task:** Create `SafetyController` with `AtomicBool` Kill Switch.
    *   *Acceptance Criteria:* `check_target()` returns false instantly when Kill Switch atom is set.
*   [ ] **Task:** Implement UDP Broadcast Listener for HALT signal.
    *   *Acceptance Criteria:* Listens on designated port; validates packet signature; triggers Kill Switch < 1ms.
*   [ ] **Task:** Write Unit Tests for IP range validation logic.
    *   *Acceptance Criteria:* Edge cases (CIDR boundaries, 0.0.0.0, multicast) covered 100%.

### S1.2: AF_XDP Capture Engine (25 pts)
*   [ ] **Task:** Create `Umem` abstraction for memory management.
    *   *Acceptance Criteria:* Allocates aligned memory pages; registers with kernel correctly.
*   [ ] **Task:** Implement `FillQueue` and `CompQueue` producers/consumers.
    *   *Acceptance Criteria:* No ring buffer overflows; pointers advance correctly under load.
*   [ ] **Task:** Write eBPF C code (`kern.c`) for packet filtering.
    *   *Acceptance Criteria:* Drops packets not matching CIDR map; redirects valid packets to XSK map.
*   [ ] **Task:** Implement User-space eBPF loader (`libbpf-rs` integration).
    *   *Acceptance Criteria:* Loads `.o` file; attaches to interface; pins maps.
*   [ ] **Task:** Benchmark Ring Buffer read speeds.
    *   *Acceptance Criteria:* Achieves > 1M pps processing on single core.

### S1.3: Passive Analysis (20 pts)
*   [ ] **Task:** Implement Zero-Copy Ethernet/IP Parser.
    *   *Acceptance Criteria:* Extracts SRC/DST/PROTO without `memcpy`; safe bounds checking.
*   [ ] **Task:** Implement TCP Option Extractor (MSS, Scale, Timestamp).
    *   *Acceptance Criteria:* Correctly parses Type-Length-Value options; handles NOPs.
*   [ ] **Task:** Create `AssetGraph` data structure (Petgraph).
    *   *Acceptance Criteria:* Nodes = IPs; Edges = Conversations; thread-safe updates.
*   [ ] **Task:** Implement TUI Dashboard skeleton (Crossterm).
    *   *Acceptance Criteria:* Responsive UI; renders empty tables/graphs; handles window resize.

---

## Phase 2: Active Stealth & Mimicry (Weeks 44-47)

### S2.1: Active Probing Engine (25 pts)
*   [ ] **Task:** Implement raw packet construction (SYN, ACK, UDP).
    *   *Acceptance Criteria:* Checksums calculated correctly (IPv4/TCP); valid headers.
*   [ ] **Task:** Create `ConnectionTable` (Stateless Hash Map).
    *   *Acceptance Criteria:* Tracks `(SrcIP, SrcPort, DstIP, DstPort)` -> `State`; auto-expires old entries.
*   [ ] **Task:** Implement Pareto Distribution RNG for Jitter.
    *   *Acceptance Criteria:* Distribution matches parameters (Alpha); high entropy.
*   [ ] **Task:** Implement "Inverse Mapping" logic (interpreting ICMP Unreach).
    *   *Acceptance Criteria:* Distinguishes "Filtered" (No response) vs "Closed" (RST/ICMP Type 3 Code 3).

### S2.2: Mimicry Profiles (25 pts)
*   [ ] **Task:** Implement Base32 DNS Encoder/Decoder.
    *   *Acceptance Criteria:* RFC 4648 compliant; case-insensitive decoding.
*   [ ] **Task:** Implement DNS Packet Builder (Header + Question).
    *   *Acceptance Criteria:* Recursion Desired flag togglable; random Transaction ID.
*   [ ] **Task:** Implement ICMP Payload steganography logic.
    *   *Acceptance Criteria:* Hides data in padding; calculates correct checksum.
*   [ ] **Task:** Implement TLS Client Hello generator (Chrome fingerprint).
    *   *Acceptance Criteria:* Matches JA3 hash of Chrome 120; valid extensions/ciphers.
*   [ ] **Task:** Verify generated packets against Wireshark dissectors.
    *   *Acceptance Criteria:* No "Malformed Packet" errors in Wireshark.

---

## Phase 3: Exfiltration & Reporting (Weeks 48-51)

### S3.1: Exfiltration Logic (25 pts)
*   [ ] **Task:** Create `SyntheticDataGen` (Luhn algorithm for CCs).
    *   *Acceptance Criteria:* Generates valid-looking SSNs, CCs; configurable volume.
*   [ ] **Task:** Implement `JobScheduler` for chunk transmission.
    *   *Acceptance Criteria:* Queues chunks; retries failed chunks; respects rate limit.
*   [ ] **Task:** Implement `Splitter` logic for Multi-Path routing.
    *   *Acceptance Criteria:* Packet 1 -> DNS, Packet 2 -> ICMP; sequence numbers preserved.
*   [ ] **Task:** Implement Rate Limiter (Token Bucket).
    *   *Acceptance Criteria:* Precise bandwidth control (+/- 5%); bursts allowed up to limit.

### S3.2: User Interface & Reporting (20 pts)
*   [ ] **Task:** Connect AssetDB to TUI Graph Widget.
    *   *Acceptance Criteria:* Visualizes nodes; color codes by OS type.
*   [ ] **Task:** Implement PCAPNG Writer (using `pcap-file` crate).
    *   *Acceptance Criteria:* Valid PCAPNG output; readable by Wireshark; correct timestamps.
*   [ ] **Task:** Implement JSON Findings Export.
    *   *Acceptance Criteria:* Schema validation passes; includes all discovered assets.

### S3.3: Integration & QA (25 pts)
*   [ ] **Task:** Build Docker Lab (Victim, Firewall, Attacker).
    *   *Acceptance Criteria:* Reproducible test environment; automated startup.
*   [ ] **Task:** Run End-to-End Exfil Tests.
    *   *Acceptance Criteria:* File transfer successful checksum match; evasion metrics recorded.
*   [ ] **Task:** Perform Governance Fuzzing.
    *   *Acceptance Criteria:* No crashes; no leaks; 100% block rate for out-of-scope.
*   [ ] **Task:** Write User Manual / Man Pages.
    *   *Acceptance Criteria:* Covers all CLI flags; includes examples; troubleshooting section.