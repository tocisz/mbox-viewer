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

### Option 1: Docker (Recommended)
You can run the application immediately using Docker. This pulls the latest image from DockerHub.

**1. Indexing (First Run):**
Run this command once to parse your emails. The container will exit automatically when finished.
```bash
docker run --rm \
  -v /path/to/your/mail.mbox:/data/mail.mbox \
  -v mbox-data:/data \
  tocisz/mbox-viewer:latest
```
*Note: Replace `/path/to/your/mail.mbox` with the actual path to your MBOX file.*

**2. Start the Viewer:**
Start the application in the background:
```bash
docker run -d --rm -p 8001:8001 \
  --name mbox-viewer \
  -v mbox-data:/data \
  tocisz/mbox-viewer:latest
```

**3. Access the App:**
Open [http://localhost:8001](http://localhost:8001) in your browser.

**4. Stop the Viewer:**
```bash
docker stop mbox-viewer
```

### Option 2: Binary Download
Download the latest standalone binary from the [GitHub Releases](https://github.com/tocisz/mbox-viewer/releases) page.

1. Download `mbox-viewer` for your platform.
2. Make it executable: `chmod +x mbox-viewer`
3. Run the indexing command first:
   ```bash
   ./mbox-viewer index --mbox path/to/mail.mbox
   ```
4. Start the server:
   ```bash
   ./mbox-viewer serve
   ```

## Development & Advanced Usage

If you want to modify the code, build from source, or understand how the project is structured, check out the documentation folder:

- [Development Guide](docs/development.md): Setup, build from source, running tests.
- [Release Guide](docs/release.md): Building static binaries, Docker release process.
