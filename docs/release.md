# Release & Deployment

This document covers how to build the application for release, including single-binary distribution and automated Docker builds.

## Building for Distribution

You can build a single executable that contains both the backend and the frontend assets. This is used for the GitHub Release artifacts.

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

### 3. Static Linking (Legacy/Alpine Support)
To create a fully static binary (no external usage of libc) that runs on any Linux distribution (like Alpine):

1. **Prerequisites**: Install `musl-tools`:
   ```bash
   sudo apt-get install musl-tools
   ```
2. **Add Target**:
   ```bash
   rustup target add x86_64-unknown-linux-musl
   ```
3. **Build**:
   ```bash
   cd backend
   cargo build --release --target x86_64-unknown-linux-musl --features embed_frontend
   ```

## Automated Releases (GitHub Actions)

This repository uses GitHub Actions to automate releases.

- **CI Checks**: Triggered on every push and pull request.
- **Releases**: Triggered when a tag starting with `v` is pushed (e.g., `v1.0.0`).

### Repository Secrets
To enable the DockerHub build, configure the following **Repository Secrets**:
1. `DOCKER_USERNAME`: Your DockerHub username.
2. `DOCKER_PASSWORD`: Your DockerHub password or Access Token.

### How to Release
1. Ensure all changes are merged to `main`.
2. Create and push a tag:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```
3. The workflow will:
   - Build and test Frontend/Backend.
   - Build static binaries for Linux, Windows, and macOS (Intel & Apple Silicon), attaching them as zip files (e.g., `mbox-viewer-0.1.2-linux-amd64.zip`) to the GitHub Release.
   - Build and push a multi-arch Docker image (`linux/amd64`, `linux/arm64`, `linux/arm/v7`) to DockerHub (`<user>/mbox-viewer:latest` and `<user>/mbox-viewer:v1.0.0`).
