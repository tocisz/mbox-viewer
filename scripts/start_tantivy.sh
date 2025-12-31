#!/bin/bash
# start_tantivy.sh - Starts the Rust-based search-service

PORT=8001
SERVICE_DIR="search-service"
BINARY="./target/release/search-service"

# Check if something is already running on port 8001
if curl -s http://localhost:$PORT/health > /dev/null; then
    echo "Search service is already running on http://localhost:$PORT"
else
    echo "Starting Search service (Tantivy)..."
    
    cd "$SERVICE_DIR" || exit
    
    # Build if binary is missing
    if [ ! -f "$BINARY" ]; then
        echo "Binary not found, building..."
        cargo build --release
    fi
    
    # Run in background
    nohup "$BINARY" > search_service.log 2>&1 &
    
    echo "Search service started in background. Logs: $SERVICE_DIR/search_service.log"
    echo "Use 'curl http://localhost:$PORT/health' to check status."
fi
