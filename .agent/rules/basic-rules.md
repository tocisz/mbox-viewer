---
trigger: always_on
---

# Project Rules & Guidelines

This document outlines the architecture, workflows, and conventions for the Gmail MBOX Viewer project.

## 1. Architecture Overview
The project has migrated from a Python-only backend to a **Rust-based** architecture for performance and type safety.

- **Frontend**: React + Vite (`/frontend`).
  - Connects to: `http://localhost:8001` (Rust Backend).
  - Port: `5173`.
  - configuration: `frontend/src/hooks.js`.

- **Backend (Active)**: Rust (`/email-server`).
  - Binary: `email-server`.
  - Roles: API Provider, Search Engine (Tantivy), Static File Server (Attachments).
  - Port: `8001` (Default).
  - Environment Variables: `PORT`, `ATTACHMENTS_DIR`.

- **Indexer**: Rust (Inside `email-server`).
  - Role: Parses MBOX files and pushes documents to the Rust Backend's `/index` endpoint.

- **Legacy**: Python Server (`/backend/server.py`).
  - Status: **Deprecated**. Do not add new features here.
  - Port: `8000`.

## 2. Development Workflow

### Running the Application
**ALWAYS** use the Rust stack script. Do not use `run_dev.sh` unless debugging legacy code.

```bash
./scripts/run_dev_rust.sh
```

### Making Backend Changes
1. Modify Rust code in `email-server/src/main.rs`.
2. If changing the Index schema, you **must**:
   - Update `email-server/src/main.rs` (Schema definition).
   - Update `backend/indexer.py` (JSON document structure).
   - Re-index data to verify.

## 3. Testing Guidelines

### Integration Tests
We use Python `pytest` to verify the Rust backend.
- File: `backend/tests/integration_tests.py`.
- **Behavior**:
  - Starts a **fresh** instance of `email-server` on Port **8002**.
  - Uses a temporary directory for the index.
  - Indexes `backend/tests/data/sample.mbox`.
  - runs assertions against the API.

**To Run Tests:**
```bash
./.agent/workflows/run_tests.md
# OR directly:
.venv/bin/pytest backend/tests/integration_tests.py
```

### Test Data
- Do not use real MBOX data for commited tests.
- Add cases to `backend/tests/data/sample.mbox`.
- If tracking new MBOX files, ensure `.gitignore` has an exception (e.g., `!backend/tests/data/*.mbox`).

## 4. Common Pitfalls / "Gotchas"
- **Ports**: 
  - `8000`: Legacy Python (Avoid).
  - `8001`: Dev Rust Server.
  - `8002`: Integration Test Server.
- **Date Parsing**: 
  - **Indexer (Rust)**: parses email headers (`Date`, `Received`) into ISO 8601 strings. If an email has a missing/malformed date, fix it in `src/indexer.rs`.
  - **Backend (Rust)**: parses search query parameters (`start_date`, `end_date`) for filtering. Supports `YYYY-MM-DD` and RFC3339.
- **Attachments**: The backend serves attachments from a directory. The Indexer extracts them. If you change extraction logic, you must Re-index.
