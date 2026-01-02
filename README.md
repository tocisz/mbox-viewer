# Gmail MBOX Viewer

A local web application to view and search emails from a Gmail MBOX export (`Takeout/Mail`).

## Prerequisites

Ensure you have the following installed:
- **Docker** (Recommended for easy run)
- **Rust & Cargo** (For source build)
- **Trunk** (`cargo install trunk`) - For building the frontend from source


---

## 1. Environment Setup

1. **Backend (Rust)**:
   Ensure Rust is installed: [https://rustup.rs/](https://rustup.rs/)

2. **Tests (Python)**:
   ```bash
   python3 -m venv .venv
   source .venv/bin/activate
   pip install -r tests/requirements.txt
   ```

3. **Frontend (Rust)**:
   ```bash
   cd frontend
   cargo binstall trunk
   # Trunk handles dependencies and build
   ```

---

## 2. Quick Start (Recommended)

The easiest way to run the application (Backend + Frontend) is using the provided script:

```bash
./scripts/run_dev_rust.sh
```
This starts:
- **Email Server (Rust)** on `http://localhost:8001`
- The frontend is served directly by the backend at the same URL.


---

## 3. Docker (Easy Install)

You can run the application using Docker without installing Rust or Python locally.

### Build the Image
```bash
docker build -t mbox-viewer .
```

### Run with an MBOX file
To automatically index and view a specific MBOX file (data persists in the `mbox-data` volume):

```bash
docker run --rm -p 8001:8001 \
  --name mbox-viewer \
  -v /path/to/your/mail.mbox:/data/mail.mbox \
  -v mbox-data:/data \
  mbox-viewer
```

### Run with existing Index
If you already have indexed data:

```bash
docker run --rm -p 8001:8001 \
  --name mbox-viewer \
  -v mbox-data:/data \
  mbox-viewer
```

The application will be available at `http://localhost:8001`.



---

## 4. Manual Indexing (Source Build)

Before you can search, you must index your MBOX data. The indexer is now built into the Rust `backend` binary.

1. **Stop the Email Server** (if running):
   The indexer requires exclusive write access to the index.

2. **Run Indexer**:
   ```bash
   cd backend
   # Replace path with your MBOX file
   # Note: --attachments-dir is required to extract and serve attachments.
   # It should match the directory served by the backend (default: "attachments")
   cargo run --release -- index --mbox "../Takeout/Mail/All mail Including Spam and Trash.mbox" --attachments-dir "../attachments"
   ```

3. **Start the Email Server**:
   ```bash
   ./scripts/run_dev_rust.sh
   ```

---

## Project Structure
*   `backend/`: Rust-based backend (API + File Serving + Search).
*   `frontend/`: Rust (Leptos) web user interface.
*   `tests/`: Integration tests and sample data.
*   `scripts/`: Utility scripts.
