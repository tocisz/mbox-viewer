import pytest
import requests
import os
import sys
import time
import subprocess
import shutil
import signal
from pathlib import Path

# Config for tests
ROOT_DIR = Path(__file__).parent.parent
TEST_PORT = 8002
API_URL = f"http://localhost:{TEST_PORT}"
SAMPLE_MBOX = ROOT_DIR / "tests" / "data" / "sample.mbox"
TEST_ATTACHMENTS_DIR = ROOT_DIR / "tests" / "data" / "attachments"

@pytest.fixture(scope="session", autouse=True)
def setup_environment():
    """
    Sets up the test environment:
    1. start backend on test port
    2. index data with attachments
    """
    # 0. Clean/Create attachments dir
    if TEST_ATTACHMENTS_DIR.exists():
        shutil.rmtree(TEST_ATTACHMENTS_DIR)
    TEST_ATTACHMENTS_DIR.mkdir(parents=True, exist_ok=True)
    
    # Clean run env
    TEST_RUN_DIR = ROOT_DIR / "tests" / "run_env"
    if TEST_RUN_DIR.exists():
        shutil.rmtree(TEST_RUN_DIR)
    TEST_RUN_DIR.mkdir(parents=True, exist_ok=True)

    # Index Data (DIRECTLY)
    print(f"Indexing data directly...")
    idx_env = os.environ.copy()
    
    binary_path = ROOT_DIR / "backend" / "target" / "release" / "backend"
    if not binary_path.exists():
        pytest.fail(f"Binary not found at {binary_path}")

    cmd = [
        str(binary_path),
        "index",
        "--mbox", str(SAMPLE_MBOX),
        "--reindex",
        "--attachments-dir", str(TEST_ATTACHMENTS_DIR)
    ]
    
    result = subprocess.run(cmd, env=idx_env, cwd=str(TEST_RUN_DIR))
    
    if result.returncode != 0:
        print(f"Indexing failed with return code {result.returncode}")
        pytest.fail("Failed to index sample data")
        
    print("Indexing complete.")

    # Start Backend Server
    print(f"\nStarting backend on port {TEST_PORT}...")
    
    env = os.environ.copy()
    env["ATTACHMENTS_DIR"] = str(TEST_ATTACHMENTS_DIR)
    env["PORT"] = str(TEST_PORT)
    env["RUST_LOG"] = "info" # Enable logging
    
    proc = subprocess.Popen(
        [str(binary_path)], 
        cwd=str(TEST_RUN_DIR), 
        env=env
    )

        
    # Wait for server to be ready
    print("Waiting for server to be ready...")
    ready = False
    for i in range(10): # Try for 5 seconds
        try:
            resp = requests.get(f"{API_URL}/health")
            if resp.status_code == 200:
                print("Server is ready!")
                ready = True
                break
        except requests.exceptions.ConnectionError:
            pass
        time.sleep(0.5)
        
    if not ready:
        proc.terminate()
        pytest.fail("Server failed to start within 5 seconds")

    yield
    
    # Cleanup
    print("Stopping backend server...")
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()

    
    if TEST_ATTACHMENTS_DIR.exists():
        shutil.rmtree(TEST_ATTACHMENTS_DIR)
    if TEST_RUN_DIR.exists():
        shutil.rmtree(TEST_RUN_DIR)


def test_health():
    resp = requests.get(f"{API_URL}/health")
    assert resp.status_code == 200
    assert resp.json() == {"status": "ok"}

def test_search_all():
    resp = requests.get(f"{API_URL}/search", params={"q": "Test", "size": 10})
    assert resp.status_code == 200
    data = resp.json()
    assert data["total"] >= 3
    
def test_search_label():
    resp = requests.get(f"{API_URL}/search", params={"label": "Important"})
    assert resp.status_code == 200
    data = resp.json()
    # Check if we found the email
    items = data["items"]
    assert any(item["subject"] == "Test Email 1" for item in items)

def test_get_email_detail():
    # Search specifically for Email 1
    resp = requests.get(f"{API_URL}/search", params={"q": "\"Test Email 1\""})
    items = resp.json()["items"]
    
    # Filter to be sure (since 'Test Email 1' contains 'Test Email' which matches others)
    target_ids = [i["id"] for i in items if i["subject"] == "Test Email 1"]
    assert len(target_ids) > 0
    email_id = target_ids[0]
    
    # Get detail
    resp = requests.get(f"{API_URL}/email/{email_id}")
    assert resp.status_code == 200
    data = resp.json()
    
    assert data["subject"] == "Test Email 1"
    assert "<pre>" in data["body_html"] or "This is a plain text email" in data["body_html"]

def test_get_email_with_attachment():
    # Search for Email 3
    resp = requests.get(f"{API_URL}/search", params={"q": "\"Test Email 3\""})
    items = resp.json()["items"]
    target_ids = [i["id"] for i in items if "Test Email 3" in i["subject"]]
    assert len(target_ids) > 0
    email_id = target_ids[0]
    
    resp = requests.get(f"{API_URL}/email/{email_id}")
    data = resp.json()
    assert len(data["attachments"]) == 1

def test_download_attachment():
    # Find email 3
    resp = requests.get(f"{API_URL}/search", params={"q": "\"Test Email 3\""})
    items = resp.json()["items"]
    target_item = next((i for i in items if "Test Email 3" in i["subject"]), None)
    assert target_item is not None
    email_id = target_item["id"]
    
    detail = requests.get(f"{API_URL}/email/{email_id}").json()
    
    # If attachments is list
    if isinstance(detail["attachments"], list):
        att_path = detail["attachments"][0]["path"]
    else:
        # If it's single object
        att_path = detail["attachments"]["path"]
        
    # Download
    resp = requests.get(f"{API_URL}/attachment/{att_path}")
    assert resp.status_code == 200
    assert b"Hello World" in resp.content # Base64 decoded content

def test_search_date_simple():
    # Test YYYY-MM-DD format
    # Sample data has emails in Dec 2025.
    
    # Search for emails BEFORE 2026-01-01 (should find them)
    resp = requests.get(f"{API_URL}/search", params={"end_date": "2026-01-01", "size": 10})
    assert resp.status_code == 200
    data = resp.json()
    assert data["total"] >= 3
    
    # Search for emails AFTER 2025-01-01 (should find them)
    resp = requests.get(f"{API_URL}/search", params={"start_date": "2025-01-01", "size": 10})
    assert resp.status_code == 200
    data = resp.json()
    assert data["total"] >= 3
    
    # Search for emails BEFORE 2024-01-01 (should find the 2014 Chat log)
    resp = requests.get(f"{API_URL}/search", params={"end_date": "2024-01-01", "size": 10})
    assert resp.status_code == 200
    data = resp.json()
    assert data["total"] >= 1
    
    # Search for emails BEFORE 2000-01-01 (should find NONE)
    resp = requests.get(f"{API_URL}/search", params={"end_date": "2000-01-01", "size": 10})
    assert resp.status_code == 200
    data = resp.json()
    assert data["total"] == 0

def test_chat_date_parsing():
    # Search for Chat Log Test
    resp = requests.get(f"{API_URL}/search", params={"q": "\"Chat Log Test\""})
    items = resp.json()["items"]
    assert len(items) > 0
    item = items[0]
    
    # Should match the date in From line/X-Received (2014-07-10)
    # The time might be UTC or local depending on parsing, but year/month/day should match
    assert "2014" in item["date"]
    assert "-07" in item["date"]
    assert "-10" in item["date"]

def test_crlf_sanitization():
    # Search for email with newline in subject
    # Note: Search query might need to be partial since we sanitized it
    resp = requests.get(f"{API_URL}/search", params={"q": "Newline"})
    items = resp.json()["items"]
    # We expect one email
    target_item = next((i for i in items if "Subject with Newline" in i["subject"]), None)
    
    # If sanitization worked, it should be "Subject with Newline" (collapsed spaces) 
    # or "Subject with  Newline" depending on exact logic, but definitely NO \n or \r
    assert target_item is not None
    assert "\n" not in target_item["subject"]
    assert "\r" not in target_item["subject"]
    
def test_mime_decoding():
    from email.header import decode_header
    
    # The subject we put in sample.mbox:
    encoded_subject = "=?iso-8859-2?Q?Zaza=F3=B3=E6_G=EA=9Cl=B1_Ja=BC=F1?="
    
    # Decode it dynamically to get the expected string
    # decode_header returns list of (bytes, encoding)
    decoded_parts = decode_header(encoded_subject)
    expected_subject = ""
    for part, encoding in decoded_parts:
        if isinstance(part, bytes):
            expected_subject += part.decode(encoding or "utf-8")
        else:
            expected_subject += part
            
    print(f"DEBUG: Expected subject: {expected_subject}")

    resp = requests.get(f"{API_URL}/search", params={"size": 50})
    items = resp.json()["items"]
    
    found = False
    for item in items:
        # Check for exact match
        if item["subject"] == expected_subject:
            found = True
            break
            
    assert found, f"Could not find email with subject: {expected_subject}"

def test_attachment_filename_sanitization():
    resp = requests.get(f"{API_URL}/search", params={"q": "\"Attachment with Slash\""})
    items = resp.json()["items"]
    assert len(items) > 0
    email_id = items[0]["id"]
    
    resp = requests.get(f"{API_URL}/email/{email_id}")
    data = resp.json()
    attachments = data["attachments"]
    assert len(attachments) == 1
    
    # Filename should be sanitized (images/slash.png -> images_slash.png or similar)
    if isinstance(attachments, list):
        att = attachments[0]
    else:
        att = attachments
        
    assert "/" not in att["filename"]
    assert "images_slash.png" in att["filename"]
    
def test_plain_text_body():
    # Check "Test Email 1" again for explicit body check
    resp = requests.get(f"{API_URL}/search", params={"q": "\"Test Email 1\""})
    items = resp.json()["items"]
    email_id = items[0]["id"]
    
    resp = requests.get(f"{API_URL}/email/{email_id}")
    data = resp.json()
    
    # Verify body_text matches content
    assert "This is a plain text email." in data["body_html"] or "This is a plain text email." in data.get("body_text", "")

def test_malformed_date_parsing():
    # 1. Short date: 28-09-12 -> 2012-09-28
    resp = requests.get(f"{API_URL}/search", params={"q": "\"Malformed Date Short\""})
    items = resp.json()["items"]
    assert len(items) > 0
    item = items[0]
    # Check if year is 2012
    assert "2012-09-28" in item["date"]
    
    # 2. Incomplete date: Wed, 14 May 2008 15 -> 2008-05-14
    resp = requests.get(f"{API_URL}/search", params={"q": "\"Malformed Date Incomplete\""})
    items = resp.json()["items"]
    assert len(items) > 0
    item = items[0]
    # Check if year is 2008
    assert "2008-05-14" in item["date"]

def test_received_header_fallback():
    # Search for "Fallback Date Received Test"
    # It has "Received: ...; Fri, 21 Nov 2007 09:55:06 -0600"
    resp = requests.get(f"{API_URL}/search", params={"q": "\"Fallback Date Received Test\""})
    items = resp.json()["items"]
    assert len(items) > 0
    item = items[0]
    
    # Should resolve to 2007
    assert "2007-11-21" in item["date"]
