# WRAITH-RedOps

**Status:** Implementation Complete (Phase 1-4 Logic Integrated)
**Version:** 2.2.5

WRAITH-RedOps is a comprehensive adversary emulation platform designed for authorized red teaming operations. It features a distributed architecture with a high-performance team server, a cross-platform implant (`Spectre`), and a modern operator console.

## 🏗️ Architecture

The platform consists of four main components:

| Component | Directory | Description |
|:---|:---|:---|
| **Team Server** | [`team-server/`](./team-server) | Central C2 controller. Rust, Axum, Tonic (gRPC), PostgreSQL. Handles listener management, task queuing, and governance enforcement. |
| **Operator Client** | [`operator-client/`](./operator-client) | Operator GUI. Tauri v2, React 19, TypeScript. Provides real-time dashboard, interactive terminal, and campaign visualization. |
| **Spectre Implant** | [`spectre-implant/`](./spectre-implant) | Advanced agent. `no_std` Rust. Features sleep masking, heap encryption, and WRAITH-native secure communications. |
| **Protocol** | [`proto/`](./proto) | Shared API definitions. gRPC/Protobuf. Defines the contract between Operators, Servers, and Implants. |

## 🚀 Quick Start

The easiest way to stand up the entire infrastructure is using the provided orchestration script.

### Prerequisites
*   **Docker:** For the PostgreSQL database.
*   **Rust (Latest Stable):** For building Server and Implant.
*   **Node.js (LTS):** For the Client frontend.
*   **System Tools:** `ss` or `lsof` (for port checking).

### Automated Startup
This script handles database initialization (including conflict resolution), environment configuration, and process orchestration.

```bash
cd ..  # Go to 'clients/' directory
./start_redops.sh
```

### Manual Startup
If you prefer to run components individually:

1.  **Database:**
    ```bash
    docker run --name wraith-postgres -e POSTGRES_PASSWORD=postgres -d -p 5432:5432 postgres
    docker exec -it wraith-postgres createdb -U postgres wraith_redops
    ```

2.  **Team Server:**
    ```bash
    cd team-server
    export DATABASE_URL="postgres://postgres:postgres@127.0.0.1/wraith_redops"
    export HMAC_SECRET="dev_secret"
    export MASTER_KEY="<64-char-hex-string>"
    export GRPC_LISTEN_ADDR="0.0.0.0:50051"
    cargo run
    ```

3.  **Operator Client:**
    ```bash
    cd operator-client
    npm install
    npm run tauri dev
    ```

## 🛡️ Security & Governance

WRAITH-RedOps is built with strict governance controls for authorized engagements:
*   **Rules of Engagement (RoE):** Digitally signed scopes that restrict implant callbacks to specific CIDRs and domains.
*   **Audit Logging:** HMAC-signed immutable logs of all operator actions.
*   **Kill Switch:** Global broadcast capability to terminate all active implants immediately.

## ⚠️ Disclaimer

This tool is strictly for **authorized security testing** and **educational purposes**. Misuse of this software is a violation of federal and/or state law. The authors and contributors assume no liability for misuse.
