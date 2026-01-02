#!/bin/bash
# run_dev_rust.sh - Starts Rust Backend (email-server) and Frontend

# Navigate to the project root
cd "$(dirname "$0")/.."

# Check/Build Rust Backend
echo "Checking Rust backend..."
cd email-server
if [ ! -f "target/release/email-server" ]; then
    echo "Building email-server..."
    cargo build --release
fi
cd ..

# Trap SIGINT (Ctrl+C) to kill both background processes
trap "echo 'Stopping dev servers...'; kill 0" SIGINT

echo "Starting Email Server (Rust) on http://localhost:8001..."
# Run from email-server dir to ensure index is stored there and attachments path is correct
(cd email-server && ./target/release/email-server serve --attachments-dir "../attachments") &

# Give it a moment to bind port
sleep 2

echo "Starting Frontend (vite) on http://localhost:5173..."
(cd frontend && npm run dev) &

# Wait for background processes
wait
