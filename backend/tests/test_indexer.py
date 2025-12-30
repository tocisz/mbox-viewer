
import unittest
import os
import tempfile
import sys
from datetime import datetime

# Add parent directory to path to import indexer
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

import indexer
from indexer import stream_mbox_messages, generate_docs, clean_html, parse_labels

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

if __name__ == '__main__':
    unittest.main()
