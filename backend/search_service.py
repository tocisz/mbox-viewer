from abc import ABC, abstractmethod
from typing import List, Optional, Dict, Any, Iterable
from elasticsearch import Elasticsearch, helpers
import os
import logging

class SearchService(ABC):
    @abstractmethod
    def health_check(self) -> bool:
        pass

    @abstractmethod
    def create_index(self, index_name: str, mapping: Dict[str, Any], reindex: bool = False):
        pass

    @abstractmethod
    def index_documents(self, index_name: str, documents: Iterable[Dict[str, Any]]):
        pass

    @abstractmethod
    def search(self, index_name: str, query_body: Dict[str, Any]) -> Dict[str, Any]:
        pass

    @abstractmethod
    def get_document(self, index_name: str, doc_id: str) -> Dict[str, Any]:
        pass

    @abstractmethod
    def get_labels(self, index_name: str) -> List[str]:
        pass

class ElasticsearchService(SearchService):
    def __init__(self, host: str = "http://localhost:9200"):
        self.es = Elasticsearch(host)

    def health_check(self) -> bool:
        try:
            return self.es.ping()
        except Exception:
            return False

    def create_index(self, index_name: str, mapping: Dict[str, Any], reindex: bool = False):
        if reindex and self.es.indices.exists(index=index_name):
            logging.info(f"Deleting existing index '{index_name}' for reindexing...")
            self.es.indices.delete(index=index_name)

        if not self.es.indices.exists(index=index_name):
            self.es.indices.create(index=index_name, body=mapping)
            logging.info(f"Created index '{index_name}'")

    def index_documents(self, index_name: str, documents: Iterable[Dict[str, Any]]):
        # Elasticsearch's helpers.bulk accepts an iterator, so we just pass it along.
        helpers.bulk(self.es, documents, chunk_size=500)

    def search(self, index_name: str, query_body: Dict[str, Any]) -> Dict[str, Any]:
        return self.es.search(index=index_name, body=query_body)

    def get_document(self, index_name: str, doc_id: str) -> Dict[str, Any]:
        return self.es.get(index=index_name, id=doc_id)

    def get_labels(self, index_name: str) -> List[str]:
        query = {
            "size": 0,
            "aggs": {
                "unique_labels": {
                    "terms": {"field": "labels", "size": 1000}
                }
            }
        }
        res = self.es.search(index=index_name, body=query)
        buckets = res["aggregations"]["unique_labels"]["buckets"]
        return sorted([b["key"] for b in buckets])

class TantivyServiceBridge(SearchService):
    """
    Bridge to the future Rust-based Tantivy service.
    This will likely be an HTTP client calling a separate Rust microservice.
    """
    def __init__(self, api_base_url: str = "http://localhost:8001"):
        self.api_base_url = api_base_url
        import requests
        self.requests = requests

    def health_check(self) -> bool:
        try:
            response = self.requests.get(f"{self.api_base_url}/health")
            return response.status_code == 200
        except Exception:
            return False

    def create_index(self, index_name: str, mapping: Dict[str, Any], reindex: bool = False):
        # Implementation depends on how the Rust service handles index creation
        logging.info(f"TantivyService: Creating index {index_name} (Not implemented)")
        pass

    def index_documents(self, index_name: str, documents: Iterable[Dict[str, Any]]):
        logging.info(f"TantivyService: Indexing documents to {index_name}")
        # Tantivy service expects a list of docs in the body
        # We process the iterable in chunks to avoid memory issues and match the API
        chunk_size = 100
        current_chunk = []
        for doc in documents:
            # The doc coming from indexer.py has _source, _id, etc.
            # We need to flatten it or extract what the Rust service expects.
            # Rust service EmailDoc: {id, subject, from, to, date, labels, body_text, body_html, has_attachment, attachments}
            src = doc.get("_source", {})
            email_doc = {
                "id": doc.get("_id") or src.get("original_id"),
                "subject": src.get("subject", ""),
                "from": src.get("from", ""),
                "to": src.get("to", ""),
                "date": str(src.get("date", "")),
                "labels": src.get("labels", []),
                "body_text": src.get("body_text", ""),
                "body_html": src.get("body_html", ""),
                "has_attachment": src.get("has_attachment", False),
                "attachments": src.get("attachments", [])
            }
            current_chunk.append(email_doc)
            
            if len(current_chunk) >= chunk_size:
                resp = self.requests.post(f"{self.api_base_url}/index", json=current_chunk)
                resp.raise_for_status()
                current_chunk = []
        
        if current_chunk:
            resp = self.requests.post(f"{self.api_base_url}/index", json=current_chunk)
            resp.raise_for_status()

    def search(self, index_name: str, query_body: Dict[str, Any]) -> Dict[str, Any]:
        logging.info(f"TantivyService: Searching in {index_name}")
        response = self.requests.post(f"{self.api_base_url}/search", json=query_body)
        if response.status_code == 200:
            return response.json()
        return {"hits": {"total": {"value": 0}, "hits": []}}

    def get_document(self, index_name: str, doc_id: str) -> Dict[str, Any]:
        logging.info(f"TantivyService: Getting document {doc_id} from {index_name}")
        response = self.requests.get(f"{self.api_base_url}/doc/{doc_id}")
        if response.status_code == 200:
            return response.json()
        return {"_source": {}}

    def get_labels(self, index_name: str) -> List[str]:
        logging.info(f"TantivyService: Getting labels from {index_name}")
        response = self.requests.get(f"{self.api_base_url}/labels")
        if response.status_code == 200:
            return response.json()
        return []

def get_search_service() -> SearchService:
    service_type = os.getenv("SEARCH_SERVICE_TYPE", "elasticsearch").lower()
    
    if service_type == "elasticsearch":
        es_host = os.getenv("ES_HOST", "http://localhost:9200")
        return ElasticsearchService(es_host)
    elif service_type == "tantivy":
        tantivy_url = os.getenv("TANTIVY_API_URL", "http://localhost:8001")
        return TantivyServiceBridge(tantivy_url)
    else:
        raise ValueError(f"Unknown search service type: {service_type}")
