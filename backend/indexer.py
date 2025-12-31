import mailbox
import sys
import os
import argparse
import email
import email.policy
from bs4 import BeautifulSoup
import logging
from datetime import datetime
import re
import hashlib
from search_service import get_search_service

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

def sanitize_header(header_value):
    """
    Sanitize email header by removing CR/LF characters and decoding MIME encoding.
    
    Email headers containing carriage return (\\r) or line feed (\\n)
    violate email format specifications and cause parsing errors.
    This function also decodes RFC 2047 MIME-encoded headers like:
    =?iso-8859-2?Q?Rozpocz=EAcie_zam=F3wienia?=
    
    Args:
        header_value: Header string to sanitize, Header object, or None
        
    Returns:
        Sanitized and decoded header string, or empty string if None
    """
    if not header_value:
        return ""
    
    # Convert Header object to string (needed for compat32 policy)
    header_str = str(header_value)
    
    # Decode MIME-encoded headers (RFC 2047)
    try:
        import email.header
        decoded_parts = email.header.decode_header(header_str)
        decoded_str = ""
        for part, encoding in decoded_parts:
            if isinstance(part, bytes):
                # Decode bytes using specified encoding or utf-8 as fallback
                decoded_str += part.decode(encoding or 'utf-8', errors='replace')
            else:
                decoded_str += part
        header_str = decoded_str
    except Exception:
        # If decoding fails, use the original string
        pass
    
    # Remove CR and LF characters, replace with space
    sanitized = header_str.replace('\r', ' ').replace('\n', ' ')
    
    # Collapse multiple consecutive spaces into one
    sanitized = re.sub(r'\s+', ' ', sanitized)
    
    return sanitized.strip()


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

def extract_attachments(message, msg_id, attachments_dir=None):
    """
    Extract attachments from email message.
    
    Saves attachment files to disk and returns metadata.
    
    Args:
        message: email.message.Message object
        msg_id: Message ID (used for directory name)
        attachments_dir: Base directory for storing attachments (optional)
        
    Returns:
        List of attachment metadata dicts with keys:
        - filename: Original attachment filename
        - size: File size in bytes
        - content_type: MIME content type
        - path: Relative path to saved file
    """
    attachments = []
    
    if not attachments_dir:
        return attachments
    
    # Sanitize message ID for use as directory name
    # Remove characters that are problematic in filenames
    safe_msg_id = re.sub(r'[<>:"/\\|?*]', '_', msg_id)
    msg_dir = os.path.join(attachments_dir, safe_msg_id)
    
    if not message.is_multipart():
        return attachments
    
    for part in message.walk():
        content_disposition = str(part.get("Content-Disposition", ""))
        
        if "attachment" not in content_disposition:
            continue
            
        # Get filename
        filename = part.get_filename()
        if not filename:
            # Generate filename if not provided
            ext = part.get_content_type().split('/')[-1]
            filename = f"attachment_{len(attachments)}.{ext}"
        
        # Decode filename if it's MIME-encoded
        filename = sanitize_header(filename)
        
        # Sanitize filename further: remove path separators and stay safe
        # We replace slashes and backslashes with underscores to preserve the full name
        # but avoid directory traversal or "file not found" errors.
        filename = filename.replace('/', '_').replace('\\', '_')
        # Remove any other potentially problematic characters for the filesystem
        filename = re.sub(r'[<>:"|?*]', '_', filename)
        
        # Get content
        try:
            payload = part.get_payload(decode=True)
            if not payload:
                continue
                
            # Create message directory if it doesn't exist
            os.makedirs(msg_dir, exist_ok=True)
            
            # Save file
            file_path = os.path.join(msg_dir, filename)
            with open(file_path, 'wb') as f:
                f.write(payload)
            
            # Store metadata
            relative_path = os.path.join(safe_msg_id, filename)
            attachments.append({
                'filename': filename,
                'size': len(payload),
                'content_type': part.get_content_type(),
                'path': relative_path
            })
            
        except Exception as e:
            logging.warning(f"Failed to extract attachment '{filename}': {e}")
    
    return attachments

def parse_date(date_str):
    """
    Parse date string with multiple fallback strategies.
    
    Handles various date formats including:
    - RFC 2822 (standard email dates)
    - ISO 8601
    - Short dates (DD-MM-YY, YY-MM-DD)
    - Incomplete dates
    
    Returns datetime.now() as last resort if parsing fails.
    """
    if not date_str:
        return datetime.now()
    
    # Convert Header object to string if necessary
    if not isinstance(date_str, str):
        date_str = str(date_str)
        
    # Strip whitespace
    date_str = date_str.strip()
    if not date_str:
        return datetime.now()
    
    # Stage 1: Try standard email parser
    try:
        res = email.utils.parsedate_to_datetime(date_str)
        if isinstance(res, datetime):
            return res
    except Exception:
        pass
    
    # Stage 2: Try dateutil parser (more flexible)
    try:
        from dateutil import parser as dateutil_parser
        # Use fuzzy=True if standard parsing failed
        return dateutil_parser.parse(date_str, fuzzy=True)
    except Exception:
        pass
    
    # Stage 3: Try custom patterns for common malformed formats
    try:
        # Pattern: DD-MM-YY or YY-MM-DD (ambiguous short dates)
        match = re.match(r'^(\d{2})-(\d{2})-(\d{2})$', date_str)
        if match:
            day, month, year = match.groups()
            year_int = int(year)
            full_year = 2000 + year_int if year_int < 50 else 1900 + year_int
            return datetime(full_year, int(month), int(day))
    except Exception:
        pass
    
    # Stage 4: Last resort - return current time
    logging.warning(f"parse_date failed for '{date_str[:50]}...': All parsing strategies exhausted. Returning now()")
    return datetime.now()


def stream_mbox_messages(mbox_path):
    """
    Lazily yields email.message.Message objects from an MBOX file.
    This avoids scanning the entire file upfront which mailbox.mbox does.
    
    Uses compat32 policy to avoid strict validation of malformed headers.
    """
    with open(mbox_path, 'rb') as f:
        lines = []
        from_line = None
        for line in f:
            if line.startswith(b'From '):
                if lines:
                    # Use compat32 policy to avoid strict header validation
                    # This allows us to sanitize CR/LF later rather than failing on parse
                    msg = email.message_from_bytes(b''.join(lines), policy=email.policy.compat32)
                    if from_line:
                        msg['X-Mbox-From-Line'] = from_line.decode('utf-8', errors='replace').strip()
                    yield msg
                    lines = []
                from_line = line
            else:
                lines.append(line)
        
        # Yield the last message
        if lines:
            msg = email.message_from_bytes(b''.join(lines), policy=email.policy.compat32)
            if from_line:
                msg['X-Mbox-From-Line'] = from_line.decode('utf-8', errors='replace').strip()
            yield msg


def generate_docs(mbox_path, attachments_dir=None):
    logging.info(f"Opening MBOX file: {mbox_path} (streaming mode)")
    
    # Use streaming generator instead of mailbox.mbox
    iterator = stream_mbox_messages(mbox_path)
    
    for i, message in enumerate(iterator):
        if i % 100 == 0:
            logging.info(f"Processed {i} emails...")
            
        try:
            # Get headers and sanitize to remove CR/LF characters
            raw_msg_id = message.get("Message-ID", f"generated-{i}")
            msg_id = sanitize_header(raw_msg_id)
            if not msg_id:
                msg_id = f"generated-{i}"
            
            # Generate a short, deterministic ID (8-character hash)
            short_id = hashlib.sha256(msg_id.encode('utf-8')).hexdigest()[:12]
            
            subject = sanitize_header(message.get("Subject", ""))
            sender = sanitize_header(message.get("From", ""))
            recipients = sanitize_header(message.get("To", ""))
            
            # Date extraction with fallbacks (especially for Chat logs)
            date_str = message.get("Date")
            if not date_str:
                # Try X-Received or Received headers
                date_str = message.get("X-Received")
                if not date_str:
                    date_str = message.get("Received")
                
                # If it's a Received header, it might have "; <date>" at the end
                if date_str and ';' in str(date_str):
                    date_str = str(date_str).split(';')[-1].strip()
                
                # If still no date, use the mbox From line we preserved
                if not date_str:
                    from_line = message.get("X-Mbox-From-Line")
                    if from_line and from_line.startswith("From "):
                        # "From <addr> <day> <mon> <dd> <hh:mm:ss> <yyyy>"
                        parts = from_line.split(' ', 2)
                        if len(parts) > 2:
                            date_str = parts[2]
            
            labels = parse_labels(message.get("X-Gmail-Labels", ""))
            
            body_html, body_text = get_body(message)
            
            # If no text body, try to create one from HTML for search
            if not body_text and body_html:
                body_text = clean_html(body_html)
            
            # Extract attachments using short_id for directory
            attachments = extract_attachments(message, short_id, attachments_dir)
                
            doc = {
                "_index": "emails",
                "_id": short_id,
                "_source": {
                    "original_id": msg_id,
                    "subject": subject,
                    "from": sender,
                    "to": recipients,
                    "date": parse_date(date_str),
                    "labels": labels,
                    "body_text": body_text,
                    "body_html": body_html, # Store full HTML for display
                    "has_attachment": len(attachments) > 0,
                    "attachments": attachments
                }
            }
            yield doc
        except Exception as e:
            logging.error(f"Failed to process message {i}: {e}")



def create_index(search_service, reindex=False):
    mapping = {
        "mappings": {
            "properties": {
                "subject": {"type": "text"},
                "from": {"type": "text"},
                "to": {"type": "text"},
                "date": {"type": "date"},
                "labels": {"type": "keyword"},
                "body_text": {"type": "text"},
                "body_html": {"type": "text", "index": False}, # Changed from keyword to text, not indexed
                "has_attachment": {"type": "boolean"},
                "attachments": {
                    "type": "nested",
                    "properties": {
                        "filename": {"type": "keyword"},
                        "size": {"type": "long"},
                        "content_type": {"type": "keyword"},
                        "path": {"type": "keyword"}
                    }
                }
            }
        }
    }
    search_service.create_index(index_name="emails", mapping=mapping, reindex=reindex)


def main():
    parser = argparse.ArgumentParser(description="Index Gmail MBOX to Elasticsearch")
    parser.add_argument("--mbox", required=True, help="Path to MBOX file")
    parser.add_argument("--es-host", default="http://localhost:9200", help="Elasticsearch URL")
    parser.add_argument("--reindex", action="store_true", help="Delete and recreate the index before indexing")
    parser.add_argument("--attachments-dir", help="Directory to store attachment files")
    args = parser.parse_args()

    search_service = get_search_service()
    
    if not search_service.health_check():
        logging.error(f"Cannot connect to the search service. Is it running?")
        sys.exit(1)
        
    create_index(search_service, reindex=args.reindex)
    
    logging.info("Starting indexing...")
    try:
        search_service.index_documents("emails", list(generate_docs(args.mbox, attachments_dir=args.attachments_dir)))
        logging.info("Indexing complete.")
    except Exception as e:
        logging.error(f"Indexing failed: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
