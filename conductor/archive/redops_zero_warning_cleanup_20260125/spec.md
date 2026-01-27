# Specification: WRAITH-RedOps Final Cleanup and Completion

## 1. Overview
This track aims to achieve a "zero-warning, zero-stub" state for the WRAITH-RedOps codebase. The primary goal is to fully implement any logic that was previously stubbed, referenced but unused (dead code), or marked with "TODO". This ensures the codebase is production-ready and technically complete according to Rust strict standards.

## 2. Scope
- **Target Crates:**
    - `clients/wraith-redops/team-server`
    - `clients/wraith-redops/spectre-implant`
    - `clients/wraith-redops/operator-client`
- **Primary Objectives:**
    1.  Eliminate all `cargo check` warnings (unused imports, dead code, unused variables).
    2.  Fully implement any function currently marked as `dead_code` (e.g., `Builder::compile_implant`).
    3.  Scan for and resolve any remaining `TODO`, `FIXME`, or `unimplemented!` markers in the code.
    4.  Ensure no logic remains as a "skeleton" implementation.

## 3. Detailed Requirements

### 3.1 Team Server
- **Builder Integration:** Fully wire up `Builder::compile_implant` in `operator.rs` or relevant service to ensure it is reachable and functional.
- **Cleanup:** Remove unused imports in `listeners/smb.rs`, `services/implant.rs`, and others identified by the compiler.
- **TODO Resolution:** Scan all source files for `TODO` comments and implement the missing logic.

### 3.2 Spectre Implant
- **Warning Elimination:** Fix `unused import` warnings in `bof_loader.rs`, `injection.rs`, `socks.rs`.
- **Dead Code:** Address `unused field` warnings (e.g., `MiniHeap::heap_start`, `Task::id`, `BofLoader::raw_data`) by either using them logic-wise or removing them if truly redundant (prefer using them to enhance logic).
- **Constants:** Address unused constants in `bof_loader.rs` by using them in validation logic or removing them if incorrect.

### 3.3 Operator Client
- **Verification:** Ensure the client builds without warnings and all IPC types are fully utilized or cleaned up.

## 4. Acceptance Criteria
- `cargo check` passes with **ZERO warnings** for all RedOps crates.
- `grep -r "TODO" clients/wraith-redops` returns no functional TODOs (documentation TODOs might be acceptable if explicitly excluded, but prefer 0).
- `grep -r "unimplemented!"` returns 0 results.
- All previously "unused" functions (like `compile_implant`) are now integrated into the execution flow.
