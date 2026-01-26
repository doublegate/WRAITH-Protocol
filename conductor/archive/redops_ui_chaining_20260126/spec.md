# Track Specification: WRAITH-RedOps UI/UX Audit & Attack Chaining

## 1. Overview
This track focuses on a comprehensive audit and enhancement of the WRAITH-RedOps client application's User Interface (UI) and User Experience (UX). It encompasses all variants (CLI, TUI, and GUI) to ensure feature completeness, consistency, and usability. Additionally, it introduces an "Automated Attack Chaining" system allowing operators to design, execute, and monitor sequences of MITRE ATT&CK techniques with platform-specific visual feedback.

## 2. Scope

### 2.1 UI/UX Audit & Standardization
*   **Target Variants:**
    *   **GUI (Tauri/React):** Must adhere to **Material Design (Dark Mode)**. Inspect all menus, buttons, inputs, and outputs for consistency, responsiveness, and correct wiring.
    *   **CLI/TUI (Rust):** Must adhere to **Terminal-First Brutalism** (hacker-centric, green-on-black). Verify argument parsing, help menus, and TUI navigation.
*   **Objective:** Ensure every backend feature is exposed and usable via the UI. Verify configuration management and error feedback.

### 2.2 Automated Attack Chaining (Campaign Management)
*   **Feature:** "Chaining" MITRE ATT&CK techniques into automated sequences.
*   **GUI Implementation:** A **Drag-and-Drop Flowchart** (e.g., `reactflow`) where operators connect technique nodes to define execution flow.
*   **TUI Implementation:** A visualization of the attack chain. Operators can select between:
    *   **A) ASCII Flowchart:** 2D box-and-arrow character graphics.
    *   **B) Hierarchical Tree:** Dependency tree view.
    *   **C) Mermaid-Text:** Raw Mermaid.js syntax output.

### 2.3 Reporting & Execution Monitoring
*   **GUI Reporting:** **Real-time Status Overlay**. Update the visual nodes in the flowchart with status icons (Loading, Success, Failed) and elapsed time during execution.
*   **TUI/CLI Reporting:** **Audit Log Sidebar**. Maintain a detailed, scrollable terminal log showing raw command/response pairs for the sequence.

## 3. Functional Requirements

### GUI (Operator Console)
*   **Attack Graph Editor:** Canvas for sequence design.
*   **Technique Palette:** MITRE technique sidebar.
*   **Live Execution:** Real-time node status updates.

### TUI/CLI (Operator Console)
*   **Chain Viewer:** Command to view saved chains in selected formats.
*   **Execution Log:** Integrated log sidebar for sequence monitoring.

## 4. Acceptance Criteria
*   **Audit Pass:** No "dead" UI elements or inconsistent styling across variants.
*   **Chain Execution:** Users can design and trigger a multi-step chain via GUI/TUI.
*   **Reporting:** GUI shows live node status; TUI shows a scrollable audit log of the sequence.
