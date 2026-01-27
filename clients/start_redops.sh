#!/bin/bash
# ==============================================================================
# WRAITH-RedOps Infrastructure Launcher
# ==============================================================================
# 
# Description:
#   This script automates the deployment of the WRAITH-RedOps adversary emulation 
#   environment. It orchestrates the startup of the PostgreSQL database, the 
#   Rust-based Team Server, and the Tauri/React Operator Client.
#
#   It includes pre-flight checks for port availability (specifically 5432 for DB)
#   and handles Docker container lifecycle (create/start/resume).
#
# Components:
#   1. Database: PostgreSQL 16 (Docker container 'wraith-postgres')
#   2. Team Server: Rust/Axum (Port 8080 default, gRPC 50051)
#   3. Operator Client: Tauri/React (Dev server)
#
# Usage:
#   ./start_redops.sh
#
# ==============================================================================

set -e

# --- Configuration ---
DB_CONTAINER_NAME="wraith-postgres"
DB_PORT=5432
DB_USER="postgres"
DB_PASS="postgres"
DB_NAME="wraith_redops"
TEAM_SERVER_DIR="./wraith-redops/team-server"
CLIENT_DIR="./wraith-redops/operator-client"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Ensure we are in the correct directory (relative to the script)
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
cd "$SCRIPT_DIR"

echo -e "${GREEN}[*] Initializing WRAITH-RedOps Environment...${NC}"

# ==============================================================================
# 1. Database Initialization (PostgreSQL)
# ==============================================================================

echo -e "${YELLOW}[-] Checking Database Status...${NC}"

# Function to check if a port is in use
check_port() {
    local port=$1
    if command -v ss >/dev/null 2>&1; then
        ss -lptn "sport = :$port" | grep -q ":$port"
    elif command -v lsof >/dev/null 2>&1; then
        lsof -i :$port -sTCP:LISTEN -t >/dev/null
    elif command -v netstat >/dev/null 2>&1; then
        netstat -plnt | grep -q ":$port"
    else
        # Fallback: assume not used if tools missing, or fail?
        echo "Warning: Could not check port usage (ss/lsof/netstat missing)."
        return 1
    fi
}

# Check if port 5432 is occupied
if check_port $DB_PORT; then
    echo -e "${YELLOW}[!] Port $DB_PORT is currently in use.${NC}"
    
    # Is it our container?
    if docker ps --format '{{.Names}}' | grep -q "^${DB_CONTAINER_NAME}$"; then
        echo -e "${GREEN}[+] 'wraith-postgres' container is already running.${NC}"
    else
        # It's something else (System Postgres or another container)
        echo -e "${RED}[!] CONFLICT DETECTED: Port $DB_PORT is in use by another process.${NC}"
        
        # Check if it's a stopped docker container named differently? No, we only care if it's NOT our container running.
        # Check if it's a system service
        if systemctl is-active --quiet postgresql; then
             echo -e "${YELLOW}[!] It appears a system-level PostgreSQL service is running.${NC}"
             echo -e "${YELLOW}[!] Attempting to stop system postgresql to free the port...${NC}"
             if sudo systemctl stop postgresql; then
                 echo -e "${GREEN}[+] System PostgreSQL stopped successfully.${NC}"
             else
                 echo -e "${RED}[X] Failed to stop system PostgreSQL. Please free port $DB_PORT manually.${NC}"
                 exit 1
             fi
        else
             echo -e "${RED}[X] Unknown process bound to $DB_PORT. Please identify and stop it manually.${NC}"
             echo "    Tip: 'sudo ss -lptn | grep $DB_PORT'"
             exit 1
        fi
    fi
else
    echo -e "${GREEN}[+] Port $DB_PORT is free.${NC}"
fi

# Docker Lifecycle Management
if ! docker ps --format '{{.Names}}' | grep -q "^${DB_CONTAINER_NAME}$"; then
    if docker ps -a --format '{{.Names}}' | grep -q "^${DB_CONTAINER_NAME}$"; then
        echo -e "${YELLOW}[-] Starting existing '${DB_CONTAINER_NAME}' container...${NC}"
        docker start "$DB_CONTAINER_NAME"
    else
        echo -e "${GREEN}[+] Creating new '${DB_CONTAINER_NAME}' container...${NC}"
        docker run --name "$DB_CONTAINER_NAME" \
            -e POSTGRES_PASSWORD="$DB_PASS" \
            -d -p "$DB_PORT":"$DB_PORT" \
            postgres:latest
    fi
fi

# Verify Port Binding
echo -e "${YELLOW}[-] Verifying port binding...${NC}"
sleep 2
if ! check_port $DB_PORT; then
    echo -e "${RED}[X] CRITICAL: Container started but port $DB_PORT is NOT listening on the host.${NC}"
    echo -e "${RED}[X] Docker failed to bind the port. Attempting to fix...${NC}"
    echo -e "${YELLOW}[!] Removing broken container...${NC}"
    docker rm -f "$DB_CONTAINER_NAME"
    echo -e "${YELLOW}[!] Retrying creation...${NC}"
    docker run --name "$DB_CONTAINER_NAME" \
            -e POSTGRES_PASSWORD="$DB_PASS" \
            -d -p "$DB_PORT":"$DB_PORT" \
            postgres:latest
    sleep 2
    if ! check_port $DB_PORT; then
        echo -e "${RED}[X] Still failed to bind port $DB_PORT. Check Docker networking.${NC}"
        exit 1
    fi
    echo -e "${GREEN}[+] Port binding fixed.${NC}"
fi

# Wait for DB to be ready
echo -e "${YELLOW}[-] Waiting for PostgreSQL to accept connections...${NC}"
until docker exec "$DB_CONTAINER_NAME" pg_isready -U "$DB_USER" > /dev/null 2>&1; do
    echo -n "."
    sleep 1
done
echo -e "\n${GREEN}[+] Database is UP.${NC}"

# Create Database if not exists
echo -e "${YELLOW}[-] Ensuring database '$DB_NAME' exists...${NC}"
if ! docker exec "$DB_CONTAINER_NAME" psql -U "$DB_USER" -lqt | cut -d '|' -f 1 | grep -qw "$DB_NAME"; then
    echo -e "${GREEN}[+] Creating database '$DB_NAME'...${NC}"
    docker exec "$DB_CONTAINER_NAME" createdb -U "$DB_USER" "$DB_NAME"
else
    echo -e "${GREEN}[+] Database '$DB_NAME' already exists.${NC}"
fi

# ==============================================================================
# 2. Team Server Startup
# ==============================================================================

echo -e "${YELLOW}[-] Launching Team Server...${NC}"
if [ ! -d "$TEAM_SERVER_DIR" ]; then
    echo -e "${RED}[X] Error: Team Server directory not found at $TEAM_SERVER_DIR${NC}"
    exit 1
fi

pushd "$TEAM_SERVER_DIR" > /dev/null
export DATABASE_URL="postgres://${DB_USER}:${DB_PASS}@127.0.0.1/${DB_NAME}"
export GRPC_LISTEN_ADDR="0.0.0.0:50051"
export HTTP_LISTEN_PORT="8080"
export UDP_LISTEN_PORT="9999"
export DNS_LISTEN_PORT="5454"
export SMB_LISTEN_PORT="4445"
export HMAC_SECRET="dev_hmac_placeholder_val_1234567890"
export MASTER_KEY=$(openssl rand -hex 32)

# Build and run in background
cargo run &
SERVER_PID=$!
popd > /dev/null

echo -e "${GREEN}[+] Team Server PID: $SERVER_PID${NC}"

# ==============================================================================
# 3. Operator Client Startup
# ==============================================================================

echo -e "${YELLOW}[-] Launching Operator Client...${NC}"
if [ ! -d "$CLIENT_DIR" ]; then
    echo -e "${RED}[X] Error: Client directory not found at $CLIENT_DIR${NC}"
    kill $SERVER_PID
    exit 1
fi

pushd "$CLIENT_DIR" > /dev/null

# Install dependencies if node_modules is missing
if [ ! -d "node_modules" ]; then
    echo -e "${YELLOW}[-] Installing Node dependencies (this may take a moment)...${NC}"
    npm install
fi

echo -e "${YELLOW}[-] Starting Tauri Dev Server...${NC}"
npm run tauri dev &
CLIENT_PID=$!
popd > /dev/null

echo -e "${GREEN}[+] Client PID: $CLIENT_PID${NC}"

# ==============================================================================
# 4. Process Management & Cleanup
# ==============================================================================

# Cleanup function to kill background processes on exit
cleanup() {
    echo -e "\n${YELLOW}[!] Shutting down WRAITH-RedOps Environment...${NC}"
    if ps -p $SERVER_PID > /dev/null; then kill $SERVER_PID; fi
    if ps -p $CLIENT_PID > /dev/null; then kill $CLIENT_PID; fi
    # Optional: Stop DB container? 
    # docker stop "$DB_CONTAINER_NAME"
    echo -e "${GREEN}[+] Shutdown complete.${NC}"
}

trap cleanup EXIT INT TERM

echo -e "${GREEN}[*] Environment Running. Press Ctrl+C to stop.${NC}"

# Wait for processes
wait $SERVER_PID $CLIENT_PID
