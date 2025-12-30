# Gmail MBOX Viewer

A local web application to view and search emails from a Gmail MBOX export (`Takeout/Mail`).

## Prerequisites

The following tools are required but were not found in your environment. Please install them:

### 1. Python 3 & Pip
```bash
sudo apt update
sudo apt install python3 python3-pip python3-venv
```

### 2. Node.js & npm (for Frontend)
```bash
# Using NodeSource (recommended for newer versions)
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt install -y nodejs
```

### 3. Elasticsearch (Search Engine)
```bash
wget -qO - https://artifacts.elastic.co/GPG-KEY-elasticsearch | sudo gpg --dearmor -o /usr/share/keyrings/elasticsearch-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/elasticsearch-keyring.gpg] https://artifacts.elastic.co/packages/8.x/apt stable main" | sudo tee /etc/apt/sources.list.d/elastic-8.x.list
sudo apt-get update && sudo apt-get install elasticsearch
sudo systemctl enable elasticsearch && sudo systemctl start elasticsearch
```
*Verify it's running:* `curl localhost:9200`

## Setup

### Backend
1.  Navigate to `backend`:
    ```bash
    cd backend
    ```
2.  Install dependencies:
    ```bash
    pip3 install -r requirements.txt
    ```
3.  **Index your MBOX file** (this may take a while):
    ```bash
    python3 indexer.py --mbox "../Takeout/Mail/All mail Including Spam and Trash.mbox"
    ```
4.  Start the API server:
    ```bash
    uvicorn server:app --reload
    ```

### Frontend (To Be Implemented)
Once Node.js is installed, I will generate the React application for you.

## Project Structure
*   `backend/`: Python FastAPI server and Indexer.
*   `frontend/`: React Web UI (Coming soon).
