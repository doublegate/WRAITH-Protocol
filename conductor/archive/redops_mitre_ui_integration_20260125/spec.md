# Specification: WRAITH-RedOps MITRE ATT&CK Full-Stack Integration & Finalization

## 1. Overview
This track aims to fully integrate the MITRE ATT&CK offensive techniques into the WRAITH-RedOps platform by building the necessary UI components in the Operator Client, verifying and exposing backend logic, and performing a final comprehensive sweep of the codebase to eliminate all remaining technical debt. The goal is to provide a seamless, production-ready operator experience for executing complex tradecraft across the entire attack lifecycle.

## 2. Scope
- **Target Components:**
    - `clients/wraith-redops/operator-client` (Frontend/UI focus)
    - `clients/wraith-redops/team-server` (gRPC exposure & backend completion)
    - `clients/wraith-redops/spectre-implant` (Final logic verification)
- **Primary Objectives:**
    1.  **Backend Verification & Exposure:** Double-check all backend logic maps 1:1 to MITRE requirements and expose via gRPC.
    2.  **UI Implementation:** Build comprehensive UI panels (Phishing, Console, Persistence, Loot, Discovery) to utilize these capabilities.
    3.  **Global Cleanup:** Eliminate all remaining stubs, "TODOs", "In production" comments, mock functions, and unused code across the entire RedOps codebase.

## 3. Detailed Requirements

### 3.1 Backend Verification & Exposure (Team Server)
- **Service Enhancement:** Ensure `OperatorService` has RPC methods for:
    - Generating phishing payloads (`GeneratePhishing`).
    - Managing persistence (`ListPersistence`, `RemovePersistence`).
    - Viewing collected credentials (`ListCredentials`).
    - Querying system/network discovery data.
- **Task Dispatch:** Ensure the `implant` service correctly translates these high-level operator requests into specific tasks for the Spectre implant.
- **Cleanup:** Verify no "TODO", "FIXME", or "stub" comments remain in any backend file.

### 3.2 Operator Client UI Implementation (Frontend)
- **Phishing Builder (TA0001):**
    - Create a wizard-style UI to select payload type (HTML Smuggling, VBA Macro).
    - Provide configuration options (target filename, delivery method).
    - Handle artifact download of the generated payload.
- **Advanced Beacon Console (TA0002, TA0005, TA0007, TA0040):**
    - Enhance the `Console` component to support new command-specific workflows.
    - Implement tab completion or helper UI for `powershell`, `persist`, `dump_lsass`, `uac_bypass`, `timestomp`.
    - Provide visual feedback for technique execution success/failure.
- **Persistence Manager (TA0003):**
    - Dedicated view to track where persistence (Run keys, tasks) has been installed across the campaign.
    - Provide a "Cleanup" action to remove persistence.
- **Credential & Loot Gallery (TA0006, TA0009):**
    - Structured view for dumped credentials (LSASS, password stores).
    - Gallery view for screenshots and collected files.
- **Discovery & Recon Dashboard (TA0007):**
    - Visualize discovered systems, users, and networks in a tabular or graph format.

### 3.3 Global Codebase Sweep
- **Mandate:** Go through the rest of the WRAITH-RedOps code base (all files in `clients/wraith-redops/`).
- **Find & Fix:**
    - Any stub or skeleton implementations.
    - "In production" / "In a production" items needing completion.
    - Mock functions or arguments.
    - "TODO:" comments.
    - Incomplete code according to comments.
    - "Never used" or "unused import" warnings from `cargo`.
- **Goal:** Implement/integrate everything fully.

## 4. Acceptance Criteria
- All techniques from the MITRE ATT&CK tables (TA0001 - TA0040) are functional and triggerable from the Operator Client UI.
- No "stubs" or "placeholders" remain in the UI or Backend.
- End-to-end data flow is verified for complex tasks.
- Clean `cargo check` and `npm test` across all components with ZERO warnings or TODOs.
