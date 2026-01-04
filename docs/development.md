# Development Documentation

This guide describes how to set up the development environment, build from source, and run tests.

## Prerequisites

Ensure you have the following installed:
- **Rust & Cargo**: [https://rustup.rs/](https://rustup.rs/)
- **Trunk**: `cargo install trunk` (For building the frontend)
- **Node.js/npm**: Required for Tailwind CSS generation (backend build process)
- **Python 3**: For running integration tests

## Project Structure

*   `backend/`: Rust-based backend (API + File Serving + Search).
*   `frontend/`: Rust (Leptos) web user interface.
*   `tests/`: Integration tests and sample data.
*   `scripts/`: Utility scripts.

## Environment Setup

### 1. Backend (Rust)
Ensure Rust is installed. The backend handles API requests, serves static files, and manages the search index (Tantivy).

### 2. Frontend (Rust/Leptos)
The frontend is a WASM application built with Leptos.
```bash
cd frontend
cargo binstall trunk # or cargo install trunk
```

### 3. Tests (Python)
To run integration tests, set up a virtual environment:
```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r tests/requirements.txt
```

## Running Locally

The easiest way to run the application (Backend + Frontend) in development mode is using the provided script:

```bash
./scripts/run_dev_rust.sh
```
This script will:
1. Build the frontend (`trunk build --release`).
2. Build and start the backend server.

The application will be available at `http://localhost:8001`.

### Manual Indexing
If you need to manually index an MBOX file (e.g., for testing specific data):

1. **Stop the Server**: The indexer needs exclusive write access.
2. **Run Indexer**:
   ```bash
   cd backend
   cargo run --release -- index --mbox "/path/to/your/mail.mbox" --attachments-dir "../attachments"
   ```

## Testing

We use Python `pytest` for integration testing.

```bash
source .venv/bin/activate
pytest tests/integration_tests.py
```
Or use the helper workflow if available.
