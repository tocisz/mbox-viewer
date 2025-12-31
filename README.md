# Gmail MBOX Viewer

A local web application to view and search emails from a Gmail MBOX export (`Takeout/Mail`).

## Prerequisites

Ensure you have the following installed:
- **Python 3.10+**
- **Node.js 18+** & **npm**
- **Elasticsearch 8.17.0** (Already included in the repository)

---

## Search Service Backend

The application supports two search backends. You can choose which one to use by setting the `SEARCH_SERVICE_TYPE` environment variable.

### Option A: Elasticsearch (Legacy)
1. **Prerequisites**: [Elasticsearch 8.17.0](https://www.elastic.co/downloads/elasticsearch) (included in repo).
2. **Start**:
   ```bash
   ./scripts/start_es.sh
   ```
3. **Configure**:
   ```bash
   export SEARCH_SERVICE_TYPE=elasticsearch
   export ES_HOST=http://localhost:9200
   ```

### Option B: Tantivy (Recommended / Rust)
1. **Prerequisites**: [Rust & Cargo](https://rustup.rs/).
2. **Start**:
   ```bash
   ./scripts/start_tantivy.sh
   ```
   *This will build the service from `search-service/` and run it in the background on port 8001.*
3. **Configure**:
   ```bash
   export SEARCH_SERVICE_TYPE=tantivy
   export TANTIVY_API_URL=http://localhost:8001
   ```

---

## 2. Setup & Start Backend

The backend is built with FastAPI and runs on Python.

### Setup Environment
```bash
# Navigate to backend directory
cd backend

# Create and activate virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install dependencies
pip install -r requirements.txt
```

### Index Your MBOX File
If you haven't indexed your data yet, run the indexer. Replace the path with your actual MBOX file location.

```bash
# Ensure you are in the backend directory with .venv activated
python indexer.py --mbox "../Takeout/Mail/All mail Including Spam and Trash.mbox"
```

### Start API Server
```bash
# Ensure you are in the backend directory with .venv activated
uvicorn server:app --reload
```
The API will be available at `http://localhost:8000`.

---

## 3. Setup & Start Frontend

The frontend is a React application built with Vite.

### Setup & Run
```bash
# Navigate to frontend directory
cd frontend

# Install dependencies
npm install

# Start development server
npm run dev
```
The frontend will be available at `http://localhost:5173`.

---

## Project Structure
*   `backend/`: FastAPI server and MBOX indexer logic.
*   `frontend/`: React + Vite web user interface.
*   `search-service/`: Rust + Tantivy standalone search service.
*   `scripts/`: Utility scripts for starting services.
*   `elasticsearch-8.17.0/`: Local Elasticsearch instance.
*   `Takeout/`: (Ignored) Gmail export data.
