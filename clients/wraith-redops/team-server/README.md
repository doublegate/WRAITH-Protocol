# WRAITH-RedOps Team Server

The **Team Server** is the central brain of the WRAITH-RedOps ecosystem. It is a high-performance, asynchronous Rust application responsible for managing implants, listeners, and operator sessions.

## 🛠️ Technical Stack
*   **Core:** Rust (Tokio runtime)
*   **API:** gRPC (Tonic) via `wraith-redops-proto`
*   **HTTP/C2:** Axum
*   **Database:** PostgreSQL (SQLx)
*   **Crypto:** Noise Protocol Framework (`snow`), XChaCha20-Poly1305, Elligator2

## ⚙️ Configuration

The server is configured primarily via environment variables.

| Variable | Required | Description |
|:---|:---:|:---|
| `DATABASE_URL` | ✅ | PostgreSQL connection string (e.g., `postgres://user:pass@127.0.0.1/db`). |
| `GRPC_LISTEN_ADDR` | ✅ | Address to bind the Operator gRPC API (e.g., `0.0.0.0:50051`). |
| `HMAC_SECRET` | ✅ | Secret key for signing audit logs and session tokens. |
| `MASTER_KEY` | ✅ | **64-character hex string** (32 bytes). Used for encrypting sensitive data at rest (payloads, artifacts). |
| `HTTP_LISTEN_PORT` | ❌ | Port for HTTP C2 listeners (Default: 8080). |
| `UDP_LISTEN_PORT` | ❌ | Port for WRAITH-Native UDP listeners (Default: 9999). |

## 🏗️ Architecture

### Services
1.  **`OperatorService` (gRPC):** Handles authentication, campaign management, tasking, and real-time event streaming for the GUI client.
2.  **`ImplantService` (gRPC/HTTP):** Endpoint for implant check-ins, data exfiltration, and task retrieval.
3.  **`ListenerManager`:** Dynamic spawner for C2 listeners (HTTP, UDP, SMB, DNS).
4.  **`GovernanceEngine`:** Enforces Rules of Engagement (RoE) on every network action.

### Database Schema
The server uses `sqlx` migrations to manage the PostgreSQL schema:
*   `implants`: State and metadata of active beacons.
*   `commands`: Task queue with priority and status tracking.
*   `activity_log`: Immutable, HMAC-signed log of all actions.
*   `artifacts`: Exfiltrated files, encrypted at rest.

## 🛡️ Governance Logic
The `GovernanceEngine` (in `src/governance.rs`) enforces scope boundaries:
*   **Time Windows:** Operations are only allowed within `start_date` and `end_date`.
*   **Network Allow/Block Lists:** CIDR-based filtering for implant callbacks.
*   **Domain Scoping:** Suffix-based validation for C2 domains.

## 🚀 Development

### Database Setup
```bash
# Start DB
docker run --name wraith-postgres -e POSTGRES_PASSWORD=postgres -d -p 5432:5432 postgres
docker exec -it wraith-postgres createdb -U postgres wraith_redops

# Run Migrations (Automatic on startup, or manual:)
export DATABASE_URL="postgres://postgres:postgres@127.0.0.1/wraith_redops"
sqlx migrate run
```

### Running
```bash
# Generate a random 32-byte master key
export MASTER_KEY=$(openssl rand -hex 32)
export HMAC_SECRET="dev_secret_value"
export GRPC_LISTEN_ADDR="0.0.0.0:50051"

cargo run
```