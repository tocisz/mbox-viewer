#!/bin/bash
# run_dev.sh - Starts Backend and Frontend development servers

# Navigate to the project root
cd "$(dirname "$0")/.."

# Trap SIGINT (Ctrl+C) to kill both background processes
trap "echo 'Stopping dev servers...'; kill 0" SIGINT

echo "Starting Backend (uvicorn) on http://localhost:8000..."
(cd backend && ../.venv/bin/uvicorn server:app --reload --port 8000) &
BE_PID=$!

echo "Starting Frontend (vite) on http://localhost:5173..."
(cd frontend && npm run dev) &
FE_PID=$!

# Wait for background processes
wait
