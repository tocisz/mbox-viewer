# Gmail MBOX Viewer

[![Backend Check](https://github.com/tocisz/mbox-viewer/actions/workflows/backend-check.yml/badge.svg)](https://github.com/tocisz/mbox-viewer/actions/workflows/backend-check.yml)
[![Frontend Check](https://github.com/tocisz/mbox-viewer/actions/workflows/frontend-check.yml/badge.svg)](https://github.com/tocisz/mbox-viewer/actions/workflows/frontend-check.yml)

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

## 5. Single Binary Deployment

You can build a single executable that contains both the backend and the frontend assets. This is useful for distribution.

### 1. Build Frontend
First, build the frontend to generate the artifacts in `frontend/dist`:

```bash
cd frontend
trunk build --release
cd ..
```

### 2. Build Backend with Embedded Assets
Enable the `embed_frontend` feature:

```bash
cd backend
cargo build --release --features embed_frontend
```

The resulting binary `target/release/backend` will serve the frontend from memory.

### 3. Static Linking (Optional)
To create a fully static binary (no external usage of libc) that runs on any Linux distribution (like Alpine):

1. **Prerequisites**:
   You **MUST** install `musl-tools` (or `musl-gcc`) because some dependencies (like `ring`) require a C compiler that supports musl.
   ```bash
   sudo apt-get install musl-tools
   ```

2. Install the musl target:
   ```bash
   rustup target add x86_64-unknown-linux-musl
   ```

3. Build with the target:
   ```bash
   cd backend
   cargo build --release --target x86_64-unknown-linux-musl --features embed_frontend
   ```

   The resulting binary will be in `target/x86_64-unknown-linux-musl/release/backend`.
   Verify it with:
   ```bash
   ldd target/x86_64-unknown-linux-musl/release/backend
   # Output should encompass: "not a dynamic executable"
   ```

---

## Project Structure
*   `backend/`: Rust-based backend (API + File Serving + Search).
*   `frontend/`: Rust (Leptos) web user interface.
*   `tests/`: Integration tests and sample data.
*   `scripts/`: Utility scripts.

---

## 6. Release & Deployment

### Automated Releases
This repository uses GitHub Actions to automate releases.

- **CI Checks**: triggered on every push and pull request.
- **Releases**: triggered when a tag starting with `v` is pushed (e.g., `v1.0.0`).

### Setup Webhooks & Secrets
To enable the DockerHub build, you must configure the following **Repository Secrets** on GitHub:

1.  `DOCKER_USERNAME`: Your DockerHub username.
2.  `DOCKER_PASSWORD`: Your DockerHub password or Access Token.

### How to Release
1.  Ensure all changes are merged to `main`.
2.  Create and push a tag:
    ```bash
    git tag v1.0.0
    git push origin v1.0.0
    ```
3.  The workflow will:
    - Build and test both Frontend and Backend.
    - Build a static Linux binary (`mbox-viewer-linux-amd64`) and attach it to the Release.
    - Build and push a Docker image to DockerHub (`<user>/mbox-viewer:latest` and `<user>/mbox-viewer:v1.0.0`).
