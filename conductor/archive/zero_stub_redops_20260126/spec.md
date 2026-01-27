# Track Specification: WRAITH-RedOps Zero-Stub Completion

## 1. Overview
This track enforces a strict "Zero Stub" policy for the `WRAITH-RedOps` component. The primary goal is to revisit all source code, particularly within `clients/wraith-redops/spectre-implant/src/modules/`, and replace any remaining placeholders, "TODO" comments, mock implementations, or "stub" returns with full, production-ready logic. This includes implementing complex tradecraft like credential dumping, process injection, and network scanning using platform-appropriate techniques.

## 2. Scope
The scope encompasses all files within the `clients/wraith-redops/` directory, with a specific focus on:
*   **`spectre-implant/src/modules/`**: Ensure all modules (`credentials.rs`, `injection.rs`, `discovery.rs`, `lateral.rs`, `collection.rs`, etc.) have complete implementations for both Windows and Linux targets where applicable.
*   **`team-server/`**: Verify that any server-side handling of these new implant capabilities is also fully implemented (e.g., parsing new task types or result formats).
*   **Workflow Enforcement**: Update `conductor/workflow.md` (or similar) to explicitly forbid "stub" or "skeleton" implementations in future RedOps development.

### Specific Implementation Targets
*   **Credentials:** Full implementation of `dump_lsass` (Windows) using `MiniDumpWriteDump` or manual memory parsing.
*   **Injection:** Robust `process_hollowing` and `thread_hijack` implementations.
*   **Discovery:** Complete `net_scan` and `sys_info` implementations.
*   **Lateral Movement:** Full `psexec` and `service_stop` logic.
*   **Collection:** Complete `keylogger` functionality (if not already fully persistent).

## 3. Development Constraints
*   **Zero Stubs:** No function should return a hardcoded "Not Implemented" or "Stub" string. Every function must attempt to perform its stated action using real system calls or APIs.
*   **Platform Specifics:**
    *   **Windows:** Implementation relies on dynamic API resolution (`api_resolver`) and direct syscalls (`syscalls.rs`). Verification will be **Code Review Focus (Compilation Check)**: ensuring the code compiles (`cargo check --target x86_64-pc-windows-gnu`) and is logically sound. Runtime verification is out of scope.
    *   **Linux:** Implementation uses raw syscalls (`syscalls.rs`). Verification must include **Runtime Verification**: unit tests or integration tests that actually run on the Linux development host to confirm behavior (e.g., `sys_info` returns real data, `net_scan` actually opens a socket).
*   **Dependencies:** Continue to use `no_std` compatible approaches for the implant.

## 4. Acceptance Criteria
*   A global search for "TODO", "stub", "placeholder", "unimplemented", "mock", or hardcoded "fake" return values in `clients/wraith-redops/yields zero relevant results.
*   `clients/wraith-redops/spectre-implant` compiles without errors or warnings for `x86_64-unknown-linux-gnu`.
*   `clients/wraith-redops/spectre-implant` compiles without errors or warnings for `x86_64-pc-windows-gnu`.
*   Linux-specific implementations are verified with passing tests running on the local environment.
*   The `workflow.md` file reflects the new "Zero Stub" rule for RedOps.
