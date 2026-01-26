# Implementation Plan: WRAITH-RedOps MITRE ATT&CK Full-Stack Integration & Final Cleanup

This plan covers the end-to-end integration of offensive techniques into the Operator Client UI, backend service exposure, and a final rigorous sweep of the entire codebase to eliminate any remaining technical debt or incomplete logic.

## Phase 1: Backend Verification & gRPC Exposure [checkpoint: c3bc24e]

- [x] Task: Audit and Expose MITRE Capabilities
    - [x] Update `OperatorService` in `operator.rs` to expose `GeneratePhishing` RPC.
    - [x] Update `OperatorService` to support specific persistence and discovery queries if needed.
    - [x] Ensure all new implant tasks (powershell, persist, etc.) are correctly routed in `dispatch_tasks` in `c2/mod.rs` and `implant.rs`.
- [x] Task: Global Codebase Cleanup (Team Server)
    - [x] Scan `team-server` for any remaining "TODO", "In production", or stubs.
    - [x] Resolve any remaining unused imports or dead code warnings in `team-server`.
- [x] Task: Conductor - User Manual Verification 'Phase 1: Backend Readiness' (Protocol in workflow.md) c3bc24e

## Phase 2: Operator Client - Core UI Enhancements [checkpoint: 0866cb3]

- [x] Task: Implement Phishing Builder UI
    - [x] Create `components/PhishingBuilder.tsx` with payload configuration form.
    - [x] Integrate with `create_phishing` Tauri command.
- [x] Task: Enhance Beacon Console for Advanced Tradecraft
    - [x] Update `Console.tsx` to handle structured input/output for complex commands (e.g. PowerShell args).
    - [x] Add command helpers/autocomplete for `powershell`, `persist`, `dump_lsass`, `uac_bypass`.
- [x] Task: Implement Persistence Manager UI
    - [x] Create `components/PersistenceManager.tsx` to view/remove installed persistence.
- [x] Task: Conductor - User Manual Verification 'Phase 2: Core UI' (Protocol in workflow.md) 0866cb3

## Phase 3: Operator Client - Data & Discovery Views [checkpoint: 0ac5e14]

- [x] Task: Implement Credential & Loot Gallery
    - [x] Create `components/LootGallery.tsx` for credentials and files.
    - [x] Connect to `list_credentials` and `list_artifacts` backend endpoints.
- [x] Task: Implement Discovery Dashboard
    - [x] Create `components/DiscoveryDashboard.tsx` to visualize network scan results and system info.
- [x] Task: Global Codebase Cleanup (Operator Client)
    - [x] Scan `operator-client` for any TODOs or placeholders.
    - [x] Ensure all UI components are fully wired to real data.
- [x] Task: Conductor - User Manual Verification 'Phase 3: Data Views' (Protocol in workflow.md) 0ac5e14

## Phase 4: Implant Final Polish & Cleanup

- [~] Task: Implant Codebase Deep Clean
    - [ ] Scan `spectre-implant` for any remaining "TODO", "In production" comments.
    - [ ] Verify `no_std` compliance and zero-warning build for Windows target.
    - [ ] Ensure `task_dispatch` handles *every* implemented module correctly.
- [~] Task: Final Integration Test
    - [ ] Verify the full chain: UI -> Team Server -> Implant -> Action -> Result -> UI.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Final System Check' (Protocol in workflow.md)
