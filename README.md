# Gmail MBOX Viewer

[![Backend Check](https://github.com/tocisz/mbox-viewer/actions/workflows/backend-check.yml/badge.svg)](https://github.com/tocisz/mbox-viewer/actions/workflows/backend-check.yml)
[![Frontend Check](https://github.com/tocisz/mbox-viewer/actions/workflows/frontend-check.yml/badge.svg)](https://github.com/tocisz/mbox-viewer/actions/workflows/frontend-check.yml)
[![Integration Tests](https://github.com/tocisz/mbox-viewer/actions/workflows/integration-tests.yml/badge.svg)](https://github.com/tocisz/mbox-viewer/actions/workflows/integration-tests.yml)

A local web application to view and search emails from a Gmail MBOX export (Google Takeout). This tool allows you to browse your archived emails in a modern interface with powerful search capabilities, entirely offline.

## Use Cases
- **Browse Archives**: View emails from an old Gmail account that you've exported.
- **Search**: Perform full-text search across thousands of emails instantly.
- **View Attachments**: Access and download files attached to your emails.
- **Privacy**: No data leaves your computer; everything runs locally.

## Installation & Usage

### Option 1: Docker
You can run the application immediately using Docker. This pulls the latest image from DockerHub.

**1. Run the Viewer:**
The container now auto-detects if the index is missing and runs the indexer for you.
```bash
docker run --rm -p 8001:8001 \
  -v /path/to/your/mail.mbox:/data/mail.mbox \
  -v mbox-data:/data \
  tocisz/mbox-viewer:latest
```
*Note: Replace `/path/to/your/mail.mbox` with the actual path to your MBOX file.*
*Note: The first run will take some time to index. Subsequent runs will be instant.*



### Option 2: Binary Download
Download the latest standalone binary from the [GitHub Releases](https://github.com/tocisz/mbox-viewer/releases) page.

1. Download `mbox-viewer` for your platform.
2. Make it executable: `chmod +x mbox-viewer`
3. Run with auto-indexing:
   ```bash
   ./mbox-viewer serve --mbox-file path/to/mail.mbox
   ```
   *By default, the server listens on `127.0.0.1` (localhost). To access from other machines, use `--host 0.0.0.0`.*
   *By default, data is stored in your OS's standard data directory (e.g., `~/.local/share/mbox-viewer` on Linux).*
   *You can override this with `--index-path ./my-index` and `--attachments-dir ./my-attachments`.*
4. Or manually index first:
   ```bash
   ./mbox-viewer index --mbox path/to/mail.mbox
   ./mbox-viewer serve
   ```

### Environment Variables
- `PORT`: Port to listen on (default: `8001`).
- `ATTACHMENTS_DIR`: Directory to store attachments (default: `attachments`).
- `INDEX_PATH`: Path to Tantivy index (default: `tantivy_index`).
- `MBOX_FILE`: Path to MBOX file for auto-indexing (default: None).

## Development & Advanced Usage

If you want to modify the code, build from source, or understand how the project is structured, check out the documentation folder:

- [Development Guide](docs/development.md): Setup, build from source, running tests.
- [Release Guide](docs/release.md): Building static binaries, Docker release process.
