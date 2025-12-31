#!/bin/bash
# run_dev.sh - Starts Backend, Frontend, and Search Service

# Navigate to the project root
cd "$(dirname "$0")/.."

# Default to Tantivy
SEARCH_TYPE="tantivy"

# Parse arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --es|--elasticsearch) SEARCH_TYPE="elasticsearch" ;;
        --tantivy) SEARCH_TYPE="tantivy" ;;
        -h|--help) 
            echo "Usage: ./scripts/run_dev.sh [--es|--elasticsearch] [--tantivy]"
            exit 0
            ;;
        *) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done

# Start chosen search service
if [ "$SEARCH_TYPE" == "tantivy" ]; then
    echo "Using Tantivy search backend..."
    ./scripts/start_tantivy.sh
    export SEARCH_SERVICE_TYPE="tantivy"
    export TANTIVY_API_URL="http://localhost:8001"
else
    echo "Using Elasticsearch search backend..."
    ./scripts/start_es.sh
    export SEARCH_SERVICE_TYPE="elasticsearch"
    export ES_HOST="http://localhost:9200"
fi

# Trap SIGINT (Ctrl+C) to kill both background processes
trap "echo 'Stopping dev servers...'; kill 0" SIGINT

echo "Starting Backend (uvicorn) on http://localhost:8000 (SEARCH_SERVICE_TYPE=$SEARCH_SERVICE_TYPE)..."
(cd backend && ../.venv/bin/uvicorn server:app --reload --port 8000) &

echo "Starting Frontend (vite) on http://localhost:5173..."
(cd frontend && npm run dev) &

# Wait for background processes
wait
