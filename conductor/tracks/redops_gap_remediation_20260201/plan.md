# Implementation Plan: WRAITH-RedOps Gap Remediation & Polish

This plan follows the TDD methodology and the "Phase Completion Verification and Checkpointing Protocol" defined in `workflow.md`.

## Phase 1: Security Hardening & Tradecraft
Focus on Mesh AEAD, ROP Stack Spoofing, and Memory Zeroization.

- [x] Task: **Write Tests: Mesh AEAD Key Derivation**
    - [x] Create `spectre-implant/src/modules/test_mesh_crypto.rs`.
    - [x] Write tests to verify key derivation from a mock Campaign ID using BLAKE3.
    - [x] Write tests to verify XChaCha20-Poly1305 encryption/decryption of mesh packets.
- [x] Task: **Implement: Campaign-Derived Mesh AEAD**
    - [ ] Update `Cargo.toml` with `chacha20poly1305` if missing (verified: already in workspace).
    - [ ] Replace static XOR logic in `modules/mesh.rs` with `chacha20poly1305`.
    - [ ] Implement build-time key derivation logic in `modules/mesh.rs`.
- [ ] Task: **Write Tests: Memory Zeroization in Transform**
    - [ ] Write a test in `modules/transform.rs` using a custom wrapper to verify buffer clearing (using `zeroize`).
- [ ] Task: **Implement: Explicit Zeroization**
    - [ ] Update `modules/transform.rs` to call `.zeroize()` on the decoded buffer in `decode_base64`.
- [ ] Task: **Implement: ROP-based Stack Spoofing**
    - [ ] Refactor `utils/obfuscation.rs` to replace basic stack spoofing with a ROP chain implementation.
    - [ ] Verify functionality via existing sleep mask tests.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Security Hardening & Tradecraft' (Protocol in workflow.md)

## Phase 2: Networking & Polish
Focus on IPv6 support, timezone awareness, and comment cleanup.

- [ ] Task: **Write Tests: IPv6 C2 and SOCKS Parsing**
    - [ ] Create tests in `c2/mod.rs` for IPv6 address parsing.
    - [ ] Create tests in `modules/socks.rs` for SOCKS5 IPv6 request handling.
- [ ] Task: **Implement: IPv6 Support**
    - [ ] Update host parsing in `c2/mod.rs` to handle `[` and `]` for IPv6.
    - [ ] Update `modules/socks.rs` to handle `ATYP 0x04` (IPv6).
- [ ] Task: **Implement: Timezone-Aware Killswitch**
    - [ ] Update date parsing logic in `c2/mod.rs` to support ISO8601 with timezones.
- [ ] Task: **Implement: Documentation & Frontend Cleanup**
    - [ ] Update comments in `exfiltration.rs` and `impact.rs`.
    - [ ] Remove/standardize `console.error` in `DiscoveryDashboard.tsx` and `LootGallery.tsx`.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Networking & Polish' (Protocol in workflow.md)

## Phase 3: Testing Expansion & Final Audit
Focus on high-fidelity module testing and dependency audit.

- [ ] Task: **Write Tests: Module Coverage (Token, Transform, SideLoad, Ingress)**
    - [ ] Implement `#[cfg(target_os = "windows")]` gated tests for real API verification.
    - [ ] Implement logic-only tests that run on Linux CI.
- [ ] Task: **Execute: Dependency Audit**
    - [ ] Run `cargo deny check` and `cargo audit`.
    - [ ] Remediate any high-priority vulnerabilities.
- [ ] Task: **Implement: Mesh Security Integration Test**
    - [ ] Create a new integration test in `tests/integration_redops_mesh.rs` to verify server-implant mesh key alignment.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Testing Expansion & Final Audit' (Protocol in workflow.md)
