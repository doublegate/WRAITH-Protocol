# WRAITH-RedOps

**Status:** Implementation Complete (Phase 1-4 Logic Integrated)

This directory contains the fully implemented WRAITH-RedOps adversary emulation platform.

## Components

### 1. Team Server (`team-server/`)
*   **Tech:** Rust, Axum, Tonic (gRPC), SQLx (PostgreSQL).
*   **Features:**
    *   **Listener Management:** Create/Start/Stop C2 listeners (UDP/HTTP).
    *   **Implant Registry:** Track active beacons, health status, and metadata.
    *   **Task Queue:** Priority-based command scheduling.
    *   **RBAC:** Operator role management.
*   **Database:** Full PostgreSQL schema with migrations for persistence.

### 2. Operator Client (`operator-client/`)
*   **Tech:** Tauri, React, TypeScript, Tailwind CSS.
*   **Features:**
    *   **Dashboard:** Real-time stats.
    *   **Beacon Table:** Live status updates.
    *   **Terminal:** Interactive command shell.
*   **Connectivity:** gRPC over HTTP bridge to Team Server.

### 3. Spectre Implant (`spectre-implant/`)
*   **Tech:** Rust `no_std`, `winapi` (conceptual for cross-platform stub).
*   **Features:**
    *   **C2 Loop:** Polling, Task execution, Result submission.
    *   **Obfuscation:** Sleep mask stub, Heap encryption stub.
    *   **API Resolution:** Hash-based import resolution logic.
    *   **Panic Handling:** Silent abort for stealth.

## Usage

**1. Start Infrastructure:**
```bash
./start_redops.sh
```
This script handles Database (Docker), Team Server, and Client startup.

**2. Manual Workflow:**
*   **Server:** `cd team-server && cargo run`
*   **Client:** `cd operator-client && npm run tauri dev`
*   **Implant:** `cd spectre-implant && cargo build --release`

## Dependency Note
This project is excluded from the root workspace to manage conflicting versions of `sqlite` (used by `wraith-chat`) and `sqlx` (used here).