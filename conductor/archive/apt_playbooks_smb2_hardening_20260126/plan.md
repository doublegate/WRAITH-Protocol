# Implementation Plan: APT Playbooks & SMB2 Hardening

This plan covers the backend implementation of Playbook templates and the deep protocol overhaul of the SMB C2 channel.

## Phase 1: APT Playbook Infrastructure [checkpoint: d08ec25]

- [x] **Task: Update Database Schema for Playbooks**
    - [ ] **Implement:** Add `playbooks` table and link to `playbook_steps` (similar to attack chains).
    - [ ] **Implement:** Add a `playbook_id` field to `attack_chains` to track instantiation.
- [x] **Task: Implement Multi-Format Playbook Loader**
    - [ ] **Implement:** Create a service in Team Server to watch/load `.yaml` AND `.json` files from `playbooks/` into the database on startup.
    - [ ] **Write Tests:** Verify that both a sample YAML and a sample JSON are correctly parsed and stored.
- [ ] **Task: Playbook-to-Chain Instantiation API**
    - [ ] **Implement:** Add gRPC method `InstantiatePlaybook(PlaybookId)` that creates a new `AttackChain` from a template.
- [ ] **Task: Conductor - User Manual Verification 'Phase 1: APT Playbook Infrastructure [checkpoint: d08ec25]' (Protocol in workflow.md)**

## Phase 2: SMB2 State Machine (Spectre Implant) [checkpoint: b03f632]

- [x] **Task: Define SMB2 Packet Structures** (b03f632)
    - [ ] **Implement:** In `spectre-implant/src/modules/smb.rs`, define `#[repr(C, packed)]` structs for `SMB2Header`, `NegotiateRequest`, `SessionSetupRequest`, etc.
- [x] **Task: Implement Client Handshake Logic** (b03f632)
    - [ ] **Implement:** State machine: `Negotiate` -> `SessionSetup` -> `TreeConnect`.
    - [ ] **Implement:** Raw socket logic to send/receive these packets sequentially.
- [x] **Task: Implement Data Tunneling (Write/Read)** (b03f632)
    - [ ] **Implement:** Wrap Noise-encrypted C2 frames inside `SMB2_WRITE` and `SMB2_READ` packets targeting a named pipe (e.g., `\pipe\wraith`).
- [x] **Task: Linux Validation:** (b03f632) Create a standalone harness `test_smb2_client` to verify the state machine progresses correctly against a listener.
- [ ] **Task: Conductor - User Manual Verification 'Phase 2: SMB2 State Machine (Spectre Implant) [checkpoint: b03f632]' (Protocol in workflow.md)**

## Phase 3: SMB2 Listener (Team Server) [checkpoint: 97458ca]

- [x] **Task: Implement SMB2 Server Handshake** (97458ca)
    - [ ] **Implement:** Update `team-server/src/listeners/smb.rs` to respond to `NEGOTIATE` and `SESSION_SETUP`.
- [~] **Task: Implement Pipe Emulation - [ ] **Task: Implement Pipe Emulation & Data Extraction** Data Extraction**
    - [ ] **Implement:** Extract C2 frames from `SMB2_WRITE` payloads and respond via `SMB2_READ`.
- [ ] **Task: Conductor - User Manual Verification 'Phase 3: SMB2 Listener (Team Server) [checkpoint: 97458ca]' (Protocol in workflow.md)**
