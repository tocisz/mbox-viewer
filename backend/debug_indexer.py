
import sys
import logging
from indexer import generate_docs

# Configure logging
logging.basicConfig(level=logging.DEBUG, format='%(asctime)s - %(levelname)s - %(message)s')

mbox_path = "../Takeout/Mail/All mail Including Spam and Trash.mbox"
# mbox_path = "../Takeout/Mail/All mail Including Spam and Trash.mbox"

print(f"Testing indexer with {mbox_path}...")

try:
    generator = generate_docs(mbox_path)
    count = 0
    for doc in generator:
        count += 1
        print(f"Generated doc {count}: ID={doc['_id']}, Subject='{doc['_source']['subject']}'")
        if count >= 10:
            print("Stopping after 10 documents.")
            break
except Exception as e:
    print(f"Error during testing: {e}")
    import traceback
    traceback.print_exc()
