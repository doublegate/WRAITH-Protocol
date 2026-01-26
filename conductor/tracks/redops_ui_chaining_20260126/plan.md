# Implementation Plan: WRAITH-RedOps UI/UX Audit & Attack Chaining

This plan covers the comprehensive UI/UX audit across all RedOps variants and the implementation of the Automated Attack Chaining system with visual feedback.

## Phase 1: UI/UX Audit Phase 1: UI/UX Audit & Standardization Standardization [checkpoint: 3fbfdf9]

- [~] **Task: GUI Audit - [ ] **Task: GUI Audit & Material Dark Refactor** Material Dark Refactor**
    - [ ] Inspect all React components in `wraith-redops` frontend.
    - [ ] **Implement:** Ensure all buttons, inputs, and cards follow Material Design (Dark Mode) consistency via Tailwind.
    - [ ] **Implement:** Wire up any "dead" UI elements to their corresponding backend gRPC endpoints.
- [ ] **Task: CLI/TUI Audit & Brutalist Theme**
    - [ ] Inspect `wraith-redops` CLI/TUI code in Rust.
    - [ ] **Implement:** Standardize help menus (`--help`) and ensure consistent "Terminal-First Brutalism" (Green on Black) styling.
    - [ ] **Implement:** Add missing CLI options for recently implemented features (e.g., service control, credential dumping).
- [ ] **Task: Conductor - User Manual Verification 'Phase 1: UI/UX Audit Phase 1: UI/UX Audit & Standardization Standardization [checkpoint: 3fbfdf9]' (Protocol in workflow.md)**

## Phase 2: Attack Chain Backend Phase 2: Attack Chain Backend & Model Model [checkpoint: 07738b4]

- [x] **Task: Define Attack Chain Schema** (07738b4)
    - [ ] Create database models for `AttackChain` and `ChainStep`.
    - [ ] **Implement:** Add gRPC service definitions for saving, loading, and listing chains.
- [x] **Task: Implement Sequential Chain Executor** (07738b4)
    - [ ] Build a service in the Team Server to execute a sequence of tasks on an implant.
    - [ ] **Write Tests:** Verify the executor handles failures (e.g., stopping the chain if a required step fails).
    - [ ] **Implement:** Logic to dispatch tasks in order and collect results.
- [ ] **Task: Conductor - User Manual Verification 'Phase 2: Attack Chain Backend Phase 2: Attack Chain Backend & Model Model [checkpoint: 07738b4]' (Protocol in workflow.md)**

## Phase 3: GUI Attack Graph Editor [checkpoint: 38a24fd]

- [x] **Task: Integrate ReactFlow for Chain Canvas** (38a24fd)
    - [ ] Set up `reactflow` in the Operator Console.
    - [ ] **Implement:** Create technique node palette and drag-and-drop logic.
- [x] **Task: Implement Visual Execution Monitor** (38a24fd)
    - [ ] Create a "Real-time Status Overlay" for the graph nodes.
    - [ ] **Implement:** Update node colors (Green/Red/Yellow) based on live gRPC event stream data.
- [ ] **Task: Conductor - User Manual Verification 'Phase 3: GUI Attack Graph Editor [checkpoint: 38a24fd]' (Protocol in workflow.md)**

## Phase 4: TUI Attack Chain Visualization

- [ ] **Task: Implement TUI Chain Viewers**
    - [ ] **Implement:** ASCII Flowchart renderer (2D box-and-arrow).
    - [ ] **Implement:** Hierarchical Tree renderer.
    - [ ] **Implement:** Mermaid-Text output generator.
- [ ] **Task: Integrate Audit Log Sidebar**
    - [ ] **Implement:** Create a scrollable log component in the TUI session view for chain monitoring.
- [ ] **Task: Conductor - User Manual Verification 'Phase 4: TUI Attack Chain Visualization' (Protocol in workflow.md)**
