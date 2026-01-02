---
trigger: always_on
---

# Project Rules & Guidelines

This document outlines the architecture, workflows, and conventions for the Gmail MBOX Viewer project (Full Stack Rust).

## 1. Architecture Overview
The project is a **Full Stack Rust** application.

- **Frontend**: Rust (Leptos) (`/frontend-rust`).
  - **Framework**: Leptos (CSR - Client Side Rendering).
  - **Build Tool**: `trunk`.
  - **Styling**: Tailwind CSS (via `npm` & `tailwindcss` CLI).
  - **Serving**: Compiled to WebAssembly (WASM) & JS, served by the Backend.

- **Backend (Active)**: Rust (`/email-server`).
  - Binary: `email-server`.
  - Roles: 
    - API Provider. 
    - Search Engine (Tantivy).
    - Static File Server (`frontend-rust/dist` + `attachments`).
  - Port: `8001` (Default).
  - Environment Variables: `PORT`, `ATTACHMENTS_DIR`.

- **Indexer**: Rust (Inside `email-server`).
  - Role: Parses MBOX files and directly writes to the Tantivy index on the filesystem.
  - **Constraint**: Cannot run while the server is running (Single-writer lock).

## 2. Development Workflow

### Prerequisites
- **Rust & Cargo**: Latest stable.
- **Trunk**: `cargo install trunk`.
- **Node.js/npm**: Required for Tailwind CSS generation during build.

### Running the Application
**ALWAYS** use the provided script to build and run both parts.

```bash
./scripts/run_dev_rust.sh
```

This script will:
1. Build the frontend using `trunk build --release`.
2. Build the backend using `cargo build --release`.
3. Start the `email-server` on port `8001`.

### modifying the Frontend
- **Logic**: Edit Rust files in `frontend-rust/src/`.
- **UI/CSS**: proper Leptos components + Tailwind classes.
- **Build**: `trunk build` (or rely on the script).

### Making Backend Changes
1. Modify Rust code in `email-server/src/main.rs` or `email-server/src/store.rs`.
2. If changing the Index schema, you **must**:
   - Update `email-server/src/store.rs` (Schema definition).
   - Re-index data to verify.

## 3. Testing Guidelines

### Integration Tests
We use Python `pytest` to verify the Rust backend API.
- File: `tests/integration_tests.py`.
- **Behavior**:
  - Starts a **fresh** instance of `email-server` on Port **8002**.
  - Uses a temporary directory for the index.
  - Indexes `tests/data/sample.mbox`.
  - runs assertions against the API.

**To Run Tests:**
```bash
./.agent/workflows/run_tests.md
# OR directly:
.venv/bin/pytest tests/integration_tests.py
```

### Test Data
- Do not use real MBOX data for commited tests.
- Add cases to `tests/data/sample.mbox`.

## 4. Common Pitfalls / "Gotchas"
- **Ports**: 
  - `8001`: Main Application (Frontend + Backend).
  - `8002`: Integration Test Server.
- **Frontend Serving**: The backend serves `index.html` as a fallback for unknown routes to support SPA routing (if enabled). ensure `frontend-rust/dist` exists.
