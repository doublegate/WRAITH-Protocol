# WRAITH-RedOps Team Server

## Setup

1.  **Database:** Ensure PostgreSQL is running.
    ```bash
    docker run --name wraith-postgres -e POSTGRES_PASSWORD=postgres -d -p 5432:5432 postgres
    createdb -h localhost -U postgres wraith_redops
    ```

2.  **Environment:**
    ```bash
    export DATABASE_URL="postgres://postgres:postgres@localhost/wraith_redops"
    ```

3.  **Run:**
    ```bash
    cargo run
    ```

## Architecture

The Team Server exposes a gRPC interface on port `50051`.
- `OperatorService`: For the Tauri client UI.
- `ImplantService`: For C2 callbacks (typically proxied via HTTP/DNS redirectors).
