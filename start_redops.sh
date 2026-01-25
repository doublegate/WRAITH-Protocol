#!/bin/bash
set -e

echo "[*] Starting WRAITH-RedOps Environment..."

# 1. Start Database (Docker)
if ! docker ps | grep -q wraith-postgres; then
    echo "[*] Starting PostgreSQL..."
    docker run --name wraith-postgres -e POSTGRES_PASSWORD=postgres -d -p 5432:5432 postgres
    sleep 3
    echo "[*] Creating Database..."
    docker exec -it wraith-postgres createdb -U postgres wraith_redops || true
fi

# 2. Build and Run Team Server
echo "[*] Starting Team Server..."
cd clients/wraith-redops/team-server
export DATABASE_URL="postgres://postgres:postgres@localhost/wraith_redops"
cargo run &
SERVER_PID=$!
cd ../../..

# 3. Start Operator Client
echo "[*] Starting Operator Client..."
cd clients/wraith-redops/operator-client
npm install
npm run tauri dev &
CLIENT_PID=$!

# Cleanup
trap "kill $SERVER_PID $CLIENT_PID" EXIT

wait
