from fastapi import FastAPI, HTTPException, Query
from fastapi.responses import FileResponse
from elasticsearch import Elasticsearch
from pydantic import BaseModel
from typing import List, Optional
from fastapi.middleware.cors import CORSMiddleware
import os

app = FastAPI()

# Enable CORS for React frontend
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"], # In production, restrict this
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

es = Elasticsearch("http://localhost:9200")

ATTACHMENTS_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "attachments")

class Attachment(BaseModel):
    filename: str
    size: int
    content_type: str
    path: str

class EmailSummary(BaseModel):
    id: str
    subject: str
    sender: str
    date: str
    snippet: str
    labels: List[str]
    has_attachment: bool = False

class EmailDetail(BaseModel):
    id: str
    subject: str
    sender: str
    to: str
    date: str
    labels: List[str]
    body_html: str
    attachments: List[Attachment] = []

@app.get("/health")
def health():
    if not es.ping():
        raise HTTPException(status_code=503, detail="Elasticsearch not reachable")
    return {"status": "ok"}

@app.get("/labels", response_model=List[str])
def get_labels():
    # Aggregation to get unique labels
    query = {
        "size": 0,
        "aggs": {
            "unique_labels": {
                "terms": {"field": "labels", "size": 1000}
            }
        }
    }
    try:
        res = es.search(index="emails", body=query)
        buckets = res["aggregations"]["unique_labels"]["buckets"]
        # Sort labels alphabetically
        return sorted([b["key"] for b in buckets])
    except Exception as e:
        print(e)
        return []

@app.get("/search", response_model=dict)
def search_emails(
    q: Optional[str] = None,
    label: Optional[str] = None,
    page: int = 1,
    size: int = 20
):
    must_clauses = []
    
    if q:
        must_clauses.append({
            "multi_match": {
                "query": q,
                "fields": ["subject^2", "from", "body_text", "to"]
            }
        })
    
    if label:
        if label.lower() == "inbox":
             must_clauses.append({"term": {"labels": "Inbox"}})
        elif label.lower() == "sent":
             must_clauses.append({"term": {"labels": "Sent"}})
        else:
             must_clauses.append({"term": {"labels": label}})

    # Default sort by date desc
    query_body = {
        "from": (page - 1) * size,
        "size": size,
        "sort": [{"date": {"order": "desc"}}],
        "query": {"bool": {"must": must_clauses}} if must_clauses else {"match_all": {}},
        "_source": ["subject", "from", "date", "labels", "body_text", "has_attachment"] # Don't fetch full HTML for list
    }
    
    res = es.search(index="emails", body=query_body)
    
    emails = []
    for hit in res["hits"]["hits"]:
        src = hit["_source"]
        # Create a snippet from body_text
        snippet = (src.get("body_text") or "")[:200]
        emails.append({
            "id": hit["_id"],
            "subject": src.get("subject", "(No Subject)"),
            "sender": src.get("from", ""),
            "date": src.get("date", ""),
            "snippet": snippet,
            "labels": src.get("labels", []),
            "has_attachment": src.get("has_attachment", False)
        })
        
    return {
        "total": res["hits"]["total"]["value"],
        "page": page,
        "size": size,
        "items": emails
    }

@app.get("/email/{email_id}", response_model=EmailDetail)
def get_email(email_id: str):
    try:
        res = es.get(index="emails", id=email_id)
        src = res["_source"]
        return {
            "id": res["_id"],
            "subject": src.get("subject", ""),
            "sender": src.get("from", ""),
            "to": src.get("to", ""),
            "date": src.get("date", ""),
            "labels": src.get("labels", []),
            "body_html": src.get("body_html", "") or f"<pre>{src.get('body_text', '')}</pre>",
            "attachments": src.get("attachments", [])
        }
    except Exception:
        raise HTTPException(status_code=404, detail="Email not found")

@app.get("/attachment/{path:path}")
async def get_attachment(path: str):
    file_path = os.path.join(ATTACHMENTS_DIR, path)
    if not os.path.exists(file_path):
        raise HTTPException(status_code=404, detail="Attachment not found")
    
    # Security: check if the file is inside the attachments directory
    abs_attachments_dir = os.path.abspath(ATTACHMENTS_DIR)
    abs_file_path = os.path.abspath(file_path)
    if not abs_file_path.startswith(abs_attachments_dir):
        raise HTTPException(status_code=403, detail="Forbidden")

    return FileResponse(file_path, filename=os.path.basename(file_path))

