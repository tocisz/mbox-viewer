#!/bin/bash
# index_mbox.sh - Indexes an MBOX file into Elasticsearch

if [ -z "$1" ]; then
    echo "Usage: $0 <path_to_mbox_file>"
    echo "Example: $0 Takeout/Mail/All\ mail.mbox"
    exit 1
fi

MBOX_PATH="$1"
shift # Remove the first argument (mbox path) from the list

# Navigate to the project root
cd "$(dirname "$0")/.."

echo "Starting indexing of $MBOX_PATH..."
./.venv/bin/python backend/indexer.py --mbox "$MBOX_PATH" "$@"
