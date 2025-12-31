from abc import ABC, abstractmethod
from typing import List, Optional, Dict, Any
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
    def index_documents(self, index_name: str, documents: List[Dict[str, Any]]):
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

    def index_documents(self, index_name: str, documents: List[Dict[str, Any]]):
        # We assume documents already have _index, _id etc if needed for bulk, 
        # but the interface might need refinement if Tantivy handles it differently.
        # For ES, we use helpers.bulk
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

    def index_documents(self, index_name: str, documents: List[Dict[str, Any]]):
        # Forward documents to Rust service
        logging.info(f"TantivyService: Indexing {len(documents)} documents to {index_name}")
        # response = self.requests.post(f"{self.api_base_url}/index", json={"index": index_name, "docs": documents})
        pass

    def search(self, index_name: str, query_body: Dict[str, Any]) -> Dict[str, Any]:
        # Translate ES query DSL to what the Rust service expects (or just pass it along if the Rust service emulates ES)
        logging.info(f"TantivyService: Searching in {index_name}")
        # response = self.requests.post(f"{self.api_base_url}/search", json={"index": index_name, "query": query_body})
        return {"hits": {"total": {"value": 0}, "hits": []}}

    def get_document(self, index_name: str, doc_id: str) -> Dict[str, Any]:
        logging.info(f"TantivyService: Getting document {doc_id} from {index_name}")
        # response = self.requests.get(f"{self.api_base_url}/doc/{doc_id}")
        return {"_source": {}}

    def get_labels(self, index_name: str) -> List[str]:
        logging.info(f"TantivyService: Getting labels from {index_name}")
        # response = self.requests.get(f"{self.api_base_url}/labels")
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
