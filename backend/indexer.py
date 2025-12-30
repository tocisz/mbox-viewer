import mailbox
import sys
import os
import argparse
import email
import email.policy
from elasticsearch import Elasticsearch, helpers
from elasticsearch.helpers import BulkIndexError
from bs4 import BeautifulSoup
import logging
from datetime import datetime
import re

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

def clean_html(html_content):
    if not html_content:
        return ""
    soup = BeautifulSoup(html_content, "lxml")
    # Basic cleaning - user requested read-only view, so we keep most structure but maybe sanitize scripts
    for script in soup(["script", "style"]):
        script.decompose()
    return soup.get_text(" ", strip=True) # For search indexing
    # We might want to store raw HTML for display, and text for search

def parse_labels(header_value):
    if not header_value:
        return []
    # X-Gmail-Labels: Inbox,Opened,Category Promotions
    return [label.strip() for label in header_value.split(',')]

def get_body(message):
    body_html = ""
    body_text = ""
    
    if message.is_multipart():
        for part in message.walk():
            content_type = part.get_content_type()
            content_disposition = str(part.get("Content-Disposition"))
            
            if "attachment" in content_disposition:
                continue
                
            try:
                payload = part.get_payload(decode=True)
                if not payload: continue
                decoded = payload.decode(part.get_content_charset() or 'utf-8', errors='replace')
                
                if content_type == "text/html":
                    body_html += decoded
                elif content_type == "text/plain":
                    body_text += decoded
            except Exception as e:
                logging.warning(f"Error decoding part: {e}")
    else:
        try:
            payload = message.get_payload(decode=True)
            if payload:
                decoded = payload.decode(message.get_content_charset() or 'utf-8', errors='replace')
                if message.get_content_type() == "text/html":
                    body_html = decoded
                else:
                    body_text = decoded
        except Exception as e:
             logging.warning(f"Error decoding body: {e}")

    return body_html, body_text

def parse_date(date_str):
    if not date_str:
        return datetime.now()
    try:
        # parsedate_to_datetime handles many formats
        res = email.utils.parsedate_to_datetime(date_str)
        if not isinstance(res, datetime):
            logging.warning(f"parse_date returned non-datetime: {type(res)} value: {res}")
        return res
    except Exception as e:
        logging.warning(f"parse_date failed for '{date_str}': {e}. Returning now()")
        return datetime.now()

def stream_mbox_messages(mbox_path):
    """
    Lazily yields email.message.Message objects from an MBOX file.
    This avoids scanning the entire file upfront which mailbox.mbox does.
    """
    with open(mbox_path, 'rb') as f:
        lines = []
        for line in f:
            if line.startswith(b'From '):
                if lines:
                    msg = email.message_from_bytes(b''.join(lines), policy=email.policy.default)
                    yield msg
                    lines = []
            else:
                lines.append(line)
        
        # Yield the last message
        if lines:
            msg = email.message_from_bytes(b''.join(lines), policy=email.policy.default)
            yield msg

def generate_docs(mbox_path):
    logging.info(f"Opening MBOX file: {mbox_path} (streaming mode)")
    
    # Use streaming generator instead of mailbox.mbox
    iterator = stream_mbox_messages(mbox_path)
    
    for i, message in enumerate(iterator):
        if i % 100 == 0:
            logging.info(f"Processed {i} emails...")
            
        try:
            msg_id = message.get("Message-ID", f"generated-{i}")
            subject = message.get("Subject", "")
            sender = message.get("From", "")
            recipients = message.get("To", "")
            date_str = message.get("Date", "")
            labels = parse_labels(message.get("X-Gmail-Labels", ""))
            
            body_html, body_text = get_body(message)
            
            # If no text body, try to create one from HTML for search
            if not body_text and body_html:
                body_text = clean_html(body_html)
                
            doc = {
                "_index": "emails",
                "_id": msg_id,
                "_source": {
                    "subject": subject,
                    "from": sender,
                    "to": recipients,
                    "date": parse_date(date_str),
                    "labels": labels,
                    "body_text": body_text,
                    "body_html": body_html, # Store full HTML for display
                    "has_attachment": False # Placeholder, logic can be improved
                }
            }
            yield doc
        except Exception as e:
            logging.error(f"Failed to process message {i}: {e}")

def create_index(es, reindex=False):
    if reindex and es.indices.exists(index="emails"):
        logging.info("Deleting existing index 'emails' for reindexing...")
        es.indices.delete(index="emails")

    if not es.indices.exists(index="emails"):
        es.indices.create(index="emails", body={
            "mappings": {
                "properties": {
                    "subject": {"type": "text"},
                    "from": {"type": "text"},
                    "to": {"type": "text"},
                    "date": {"type": "date"},
                    "labels": {"type": "keyword"},
                    "body_text": {"type": "text"},
                    "body_html": {"type": "text", "index": False}, # Changed from keyword to text, not indexed
                    "has_attachment": {"type": "boolean"}
                }
            }
        })
        logging.info("Created index 'emails'")

def main():
    parser = argparse.ArgumentParser(description="Index Gmail MBOX to Elasticsearch")
    parser.add_argument("--mbox", required=True, help="Path to MBOX file")
    parser.add_argument("--es-host", default="http://localhost:9200", help="Elasticsearch URL")
    parser.add_argument("--reindex", action="store_true", help="Delete and recreate the index before indexing")
    args = parser.parse_args()

    es = Elasticsearch(args.es_host)
    
    if not es.ping():
        logging.error(f"Cannot connect to Elasticsearch at {args.es_host}. Is it running?")
        sys.exit(1)
        
    create_index(es, reindex=args.reindex)
    
    logging.info("Starting indexing...")
    try:
        helpers.bulk(es, generate_docs(args.mbox), chunk_size=500)
        logging.info("Indexing complete.")
    except BulkIndexError as e:
        logging.error(f"{len(e.errors)} documents failed to index. First error: {e.errors[0]}")
        sys.exit(1)

if __name__ == "__main__":
    main()
