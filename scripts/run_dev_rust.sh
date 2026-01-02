#!/bin/bash
# run_dev_rust.sh - Starts Rust Backend and Frontend (Leptos)

# Navigate to the project root
cd "$(dirname "$0")/.."

# Check for Trunk
if ! command -v trunk &> /dev/null; then
    echo "Trunk is not installed. Please install it with: cargo install trunk"
    exit 1
fi

# Build Frontend
echo "Building Frontend (Leptos)..."
cd frontend
trunk build --release
cd ..

# Build Backend
echo "Building Backend..."
cd backend
cargo build --release
cd ..

echo "Starting Email Server (Full Stack Rust) on http://localhost:8001..."
# Run from backend dir
(cd backend && ./target/release/backend serve --attachments-dir "../attachments")
