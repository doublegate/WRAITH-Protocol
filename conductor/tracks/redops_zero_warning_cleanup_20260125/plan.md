# Implementation Plan: WRAITH-RedOps Final Cleanup and Completion

This plan focuses on achieving a zero-warning and zero-stub state for all WRAITH-RedOps components, adhering to strict production-ready standards.

## Phase 1: Discovery and Baseline [checkpoint: 65e9df9]

- [x] Task: Baseline Audit - Identify all remaining warnings and TODOs
    - [x] Run `cargo check` on all three RedOps crates and document all warnings
    - [x] Run `grep -rn "TODO" clients/wraith-redops` to list all remaining implementation gaps
- [x] Task: Conductor - User Manual Verification 'Phase 1: Discovery' (Protocol in workflow.md) 65e9df9

## Phase 2: Team Server Remediation

- [~] Task: Resolve all TODOs in Team Server
    - [ ] Implement Frame header metadata in `protocol.rs` (currently marked TODO)
    - [ ] Address any other TODOs found in Phase 1
- [ ] Task: Integrate and Validate `compile_implant`
    - [ ] Ensure the function is called or exposed via an active gRPC endpoint logic
    - [ ] Fix the `dead_code` warning by providing a functional path to this logic
- [ ] Task: Clean up all Team Server warnings
    - [ ] Remove unused imports in `listeners/smb.rs` and `services/implant.rs`
    - [ ] Resolve any remaining unused variable warnings
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Team Server' (Protocol in workflow.md)

## Phase 3: Spectre Implant Remediation

- [ ] Task: Address all Spectre Implant warnings
    - [ ] Use or gate unused constants/structs in `bof_loader.rs` (e.g., `IMAGE_FILE_MACHINE_AMD64`)
    - [ ] Resolve `dead_code` for fields like `BofLoader::raw_data` and `MiniHeap::heap_start`
    - [ ] Eliminate unused import warnings in `injection.rs`, `socks.rs`, etc.
- [ ] Task: Refactor `static_mut_refs` usage
    - [ ] Update `GLOBAL_CONFIG` access in `c2/mod.rs` to use safe raw pointer access patterns to satisfy Rust 2024 requirements
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Spectre Implant' (Protocol in workflow.md)

## Phase 4: Final Validation

- [ ] Task: Global Zero-Warning Verification
    - [ ] Verify `cargo check` returns no warnings for all crates
    - [ ] Verify `grep` returns no functional TODOs or `unimplemented!` markers
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Final Validation' (Protocol in workflow.md)
