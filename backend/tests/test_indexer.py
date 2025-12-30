
import unittest
import os
import tempfile
import sys
from datetime import datetime

# Add parent directory to path to import indexer
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

import indexer
from indexer import stream_mbox_messages, generate_docs, clean_html, parse_labels, sanitize_header

class TestIndexer(unittest.TestCase):
    def setUp(self):
        # Create a temporary mbox file
        self.test_mbox_path = "temp_test.mbox"
        with open(self.test_mbox_path, "w") as f:
            f.write("From sender@example.com Tue Dec 30 12:00:00 2025\n")
            f.write("From: sender@example.com\n")
            f.write("To: recipient@example.com\n")
            f.write("Subject: Test Subject\n")
            f.write("Date: Tue, 30 Dec 2025 12:00:00 -0000\n")
            f.write("X-Gmail-Labels: Inbox,Important\n")
            f.write("\n")
            f.write("This is the body.\n")
            f.write("\n") 
            # Second message
            f.write("From other@example.com Tue Dec 30 13:00:00 2025\n")
            f.write("From: other@example.com\n")
            f.write("Subject: Another Email\n")
            f.write("\n")
            f.write("Body 2.\n")

    def tearDown(self):
        if os.path.exists(self.test_mbox_path):
            os.remove(self.test_mbox_path)

    def test_stream_mbox_messages(self):
        messages = list(stream_mbox_messages(self.test_mbox_path))
        self.assertEqual(len(messages), 2)
        self.assertEqual(messages[0]["Subject"], "Test Subject")
        self.assertEqual(messages[1]["Subject"], "Another Email")

    def test_clean_html(self):
        html = "<html><body><script>alert(1)</script><p>Hello</p></body></html>"
        text = clean_html(html)
        self.assertEqual(text, "Hello")

    def test_parse_labels(self):
        header = "Inbox, Important, Category Updates"
        labels = parse_labels(header)
        self.assertEqual(labels, ["Inbox", "Important", "Category Updates"])

    def test_generate_docs(self):
        # generate_docs yields dicts for ES
        docs = list(generate_docs(self.test_mbox_path))
        self.assertEqual(len(docs), 2)
        
        doc1 = docs[0]
        self.assertEqual(doc1["_index"], "emails")
        self.assertEqual(doc1["_source"]["subject"], "Test Subject")
        self.assertEqual(doc1["_source"]["from"], "sender@example.com")
        self.assertEqual(doc1["_source"]["labels"], ["Inbox", "Important"])
        self.assertEqual(doc1["_source"]["body_text"].strip(), "This is the body.")

    def test_parse_date(self):
        """Test parse_date with various formats and edge cases"""
        from indexer import parse_date
        
        # Test 1: Standard RFC 2822 format (should work)
        result = parse_date("Tue, 30 Dec 2025 12:00:00 -0000")
        self.assertIsInstance(result, datetime)
        self.assertEqual(result.year, 2025)
        self.assertEqual(result.month, 12)
        self.assertEqual(result.day, 30)
        
        # Test 2: Malformed date from logs - short date '28-09-12 '
        result = parse_date("28-09-12 ")
        self.assertIsInstance(result, datetime)
        # Should be parsed as DD-MM-YY: 28 Sep 2012
        self.assertEqual(result.year, 2012)
        self.assertEqual(result.month, 9)
        self.assertEqual(result.day, 28)
        
        # Test 3: Malformed date from logs - incomplete RFC date 'Wed, 14 May 2008 15'
        result = parse_date("Wed, 14 May 2008 15")
        self.assertIsInstance(result, datetime)
        # Should extract the date portion
        self.assertEqual(result.year, 2008)
        self.assertEqual(result.month, 5)
        self.assertEqual(result.day, 14)
        
        # Test 4: ISO 8601 format
        result = parse_date("2025-12-30T12:00:00Z")
        self.assertIsInstance(result, datetime)
        self.assertEqual(result.year, 2025)
        self.assertEqual(result.month, 12)
        self.assertEqual(result.day, 30)
        
        # Test 5: Empty string
        result = parse_date("")
        self.assertIsInstance(result, datetime)
        # Should return current time, so just check it's a datetime
        
        # Test 6: None
        result = parse_date(None)
        self.assertIsInstance(result, datetime)
        
        # Test 7: Whitespace only
        result = parse_date("   ")
        self.assertIsInstance(result, datetime)
        
        # Test 8: Another short date format (year in different century)
        result = parse_date("15-03-95")
        self.assertIsInstance(result, datetime)
        # Should be parsed as 15 Mar 1995 (year 95 -> 1995)
        self.assertEqual(result.year, 1995)
        self.assertEqual(result.month, 3)
        self.assertEqual(result.day, 15)
        
        # Test 9: Standard date with timezone
        result = parse_date("Wed, 14 May 2008 15:30:45 +0200")
        self.assertIsInstance(result, datetime)
        self.assertEqual(result.year, 2008)
        self.assertEqual(result.month, 5)
        self.assertEqual(result.day, 14)

    def test_sanitize_header(self):
        """Test sanitize_header with CR/LF characters and edge cases"""
        from indexer import sanitize_header
        
        # Test 1: Header with CR character
        result = sanitize_header("test@example.com\r")
        self.assertEqual(result, "test@example.com")
        
        # Test 2: Header with LF character
        result = sanitize_header("test@example.com\n")
        self.assertEqual(result, "test@example.com")
        
        # Test 3: Header with both CR and LF
        result = sanitize_header("From:\r\ntest@example.com")
        self.assertEqual(result, "From: test@example.com")
        
        # Test 4: Header with multiple CR/LF characters
        result = sanitize_header("test\r\n\r\n@example.com")
        self.assertEqual(result, "test @example.com")
        
        # Test 5: Normal header (no CR/LF)
        result = sanitize_header("test@example.com")
        self.assertEqual(result, "test@example.com")
        
        # Test 6: None input
        result = sanitize_header(None)
        self.assertEqual(result, "")
        
        # Test 7: Empty string
        result = sanitize_header("")
        self.assertEqual(result, "")
        
        # Test 8: Multiple consecutive spaces after sanitization
        result = sanitize_header("test\r\n   \r\n@example.com")
        self.assertEqual(result, "test @example.com")
        
        # Test 9: Complex Subject with newlines
        result = sanitize_header("Re: Important\nMeeting\rSchedule")
        self.assertEqual(result, "Re: Important Meeting Schedule")
        
        # Test 10: MIME-encoded header (RFC 2047) - ISO-8859-2 Polish
        result = sanitize_header("=?iso-8859-2?Q?Rozpocz=EAcie_zam=F3wienia_-_PKP_Intercity?=")
        # Should decode the MIME encoding
        self.assertIn("Rozpocz", result)
        self.assertNotIn("=?iso-8859-2?Q?", result)
        
        # Test 11: MIME-encoded header - UTF-8
        result = sanitize_header("=?UTF-8?B?VGVzdCBTdWJqZWN0?=")
        self.assertEqual(result, "Test Subject")
        
        # Test 12: Mixed MIME and plain text
        result = sanitize_header("Re: =?UTF-8?Q?Test?= Message")
        self.assertEqual(result, "Re: Test Message")


if __name__ == '__main__':
    unittest.main()
