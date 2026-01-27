# Track Specification: Comprehensive WRAITH-RedOps & Core Protocol Finalization

## 1. Overview
This track is a strict, all-encompassing "Zero Technical Debt" finalization. It enforces a "Zero Stub, Zero TODO, Zero Warning" policy across `WRAITH-RedOps`, `wraith-crypto`, and `wraith-core`. All previously deferred features (Mesh C2, full test coverage) will be fully implemented using real APIs, without shortcuts or skeletons.

## 2. Functional Requirements

### 2.1 Spectre Implant (no_std)
*   **Structured P2P Mesh C2 (High Priority):**
    *   Implement **MeshServer**: A dual TCP and SMB Named Pipe listener capable of accepting downstream beacon connections.
    *   Implement **MeshRouting**: Bidirectional logic to relay data between downstream beacons and the Team Server via `MeshRelay` frames.
    *   **Security Features:**
        *   Handshake-based authentication for downstream connections.
        *   Circular route detection to prevent frame loops.
        *   Strict encapsulation to prevent unauthorized proxying.
*   **Tradecraft Finalization:**
    *   Replace all "In production" placeholders in memory protection (Sleep Mask) and persistence modules with production-hardened logic.
*   **Zero Warning Policy:** Resolve all unused variable/import warnings identified by `cargo check`.

### 2.2 Team Server & wraith-crypto
*   **Comprehensive Integration Test Suite:**
    *   Create a test fixture that spins up an ephemeral PostgreSQL instance for automated testing.
    *   Verify all 30 gRPC methods in `OperatorService` with end-to-end integration tests.
*   **wraith-crypto no_std Refinement:**
    *   Remove all implicit `std` dependencies.
    *   Verify full compatibility with the `no_std` Spectre target without linking errors.

### 2.3 wraith-core Full Audit
*   **Protocol Remediation:**
    *   Work through `TECH-DEBT.md` and grep results to remediate ALL `TODO`, `FIXME`, and "placeholder" implementations.
    *   Specific focus on `transfer.rs` (chunking logic), `nat.rs` (ICE signaling), and `discovery.rs`.

## 3. Methodology & Workflow
1.  **Strict Rule #1:** "Zero Stubs & Zero Warnings" is now the project's immutable baseline.
2.  **Audit-First Implementation:** Use `rg` and `cargo check` to identify all debt markers before writing logic.
3.  **Real API Focus:** No mock functions; every requirement must use real OS syscalls or protocol-compliant logic.

## 4. Acceptance Criteria
*   Zero occurrences of `TODO`, `FIXME`, `placeholder`, `stub`, `skeleton`, or `In production` comments in the codebase.
*   `cargo check --workspace` returns zero warnings.
*   End-to-end verification of Mesh C2: (Child Beacon -> Parent Beacon -> Team Server).
*   100% of `OperatorService` RPC methods covered by automated integration tests.
*   Full ICE signaling confirmed working in NAT-to-NAT transfer scenarios.