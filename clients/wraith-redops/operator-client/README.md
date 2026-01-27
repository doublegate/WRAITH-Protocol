# WRAITH-RedOps Operator Client

The **Operator Client** is the mission control interface for WRAITH-RedOps. It is a modern, cross-platform desktop application built with Tauri v2, providing a secure and responsive UI for red team operators.

## 🛠️ Technical Stack

*   **Framework:** [Tauri v2](https://v2.tauri.app/) (Rust backend + Web frontend)
*   **Frontend:** React 19, TypeScript, Vite
*   **Styling:** Tailwind CSS v4
*   **Communication:** gRPC (via `tonic` in the Rust backend)
*   **State Management:** React Query + Context API

## ✨ Features

*   **Dashboard:** Real-time visualization of active campaigns, listeners, and beacon health.
*   **Beacon Management:** Interactive table for filtering and selecting implants.
*   **Terminal Interface:** Command-line style tasking for selected beacons.
*   **Campaign Manager:** Create and configure campaigns with specific Rules of Engagement (RoE).
*   **Event Stream:** Live feed of system events (check-ins, results, errors).

## 🚀 Development Setup

### Prerequisites
*   **Node.js:** LTS version (v18+ recommended)
*   **Rust:** Latest stable
*   **System Dependencies:** `libwebkit2gtk` (Linux), XCode (macOS), Visual Studio (Windows) - standard Tauri requirements.

### Installation
```bash
# Install Node dependencies
npm install
```

### Running in Dev Mode
This starts the Vite dev server and the Tauri Rust backend.
```bash
npm run tauri dev
```
*Note: Ensure the **Team Server** is running on `0.0.0.0:50051` (or configured address) before connecting.*

### Building for Release
```bash
npm run tauri build
```
The output binary will be located in `src-tauri/target/release/bundle/`.

## 🔌 Architecture
The client uses a "Sidecar" pattern for gRPC:
1.  **Frontend (React):** Handles UI logic and invokes Tauri Commands.
2.  **Backend (Rust/Tauri):** Receives commands, maintains the gRPC connection to the Team Server (via `tonic`), and streams events back to the frontend.
3.  **Proto Definitions:** Shared with the Team Server to ensure type safety.
