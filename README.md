# Gmail MBOX Viewer

A local web application to view and search emails from a Gmail MBOX export (`Takeout/Mail`).

## Prerequisites

Ensure you have the following installed:
- **Rust & Cargo** (For backend & indexing)
- **Node.js 18+** & **npm** (For frontend)
- **Rust & Cargo** (For backend)

---

## 1. Environment Setup

### Install Dependencies

1. **Backend (Rust)**:
   Ensure Rust is installed: [https://rustup.rs/](https://rustup.rs/)

2. **Indexer (Python)**:
   ```bash
   cd backend
   python3 -m venv .venv
   source .venv/bin/activate
   pip install -r requirements.txt
   ```

3. **Frontend (React)**:
   ```bash
   cd frontend
   npm install
   ```

---

## 2. Quick Start (Recommended)

The easiest way to run the application (Backend + Frontend) is using the provided script:

```bash
./scripts/run_dev_rust.sh
```
This starts:
- **Email Server (Rust)** on `http://localhost:8001`
- **Frontend** on `http://localhost:5173`

---

## 3. Index Your MBOX File

Before you can search, you must index your MBOX data. The indexer is now built into the Rust `email-server` binary.

1. **Start the Email Server**:
   ```bash
   # In a separate terminal
   cd email-server
   cargo run --release
   ```

2. **Run Indexer**:
   ```bash
   cd email-server
   # Replace path with your MBOX file
   # Note: --attachments-dir is required to extract and serve attachments. 
   # It should match the directory served by the email-server (default: "attachments")
   cargo run --release -- index --mbox "../Takeout/Mail/All mail Including Spam and Trash.mbox" --es-host "http://localhost:8001" --attachments-dir "attachments"
   ```

---

## Project Structure
*   `email-server/`: Rust-based backend (API + File Serving + Search).
*   `backend/`: Python scripts for data indexing (migrated from legacy API).
*   `frontend/`: React + Vite web user interface.
*   `scripts/`: Utility scripts.
*   `Takeout/`: (Ignored) Gmail export data.

## Legacy (Python Backend)
*The legacy Python backend (`server.py`) running on port 8000 is deprecated.*
To run the legacy stack, use: `./scripts/run_dev.sh`.
